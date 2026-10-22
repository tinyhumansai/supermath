//! The four children as real `OpenHuman` agents, played by a scripted model.
//!
//! A scripted `ChatModel` answers each child with a fixed sequence of tool
//! calls and a final reply; the primitives are fakes over an in-memory world.
//! Nothing here touches the network: the runtime's backend is a loopback stub.
//!
//! `OpenHuman` claims a process-wide slot for its runtime, so every scenario
//! runs in one test, in order, on one runtime.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// The fake world is shared with the crate's unit tests by path. Each binary
// uses a different subset of its helpers, so unused ones are expected here.
#[allow(dead_code)]
#[path = "../src/fake/world.rs"]
mod world;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use openhuman_embed::providers::{ChatModel, ModelRequest, ModelResponse};
use openhuman_embed::{Provider, Runtime, RuntimeConfig, Workspace};
use serde_json::{Value, json};
use supermath_subagents::{DuelSettings, Models, Progress, Subagents, SubagentsConfig};
use supermath_tool::{
    Budget, CallContext, Envelope, ErrorCode, InMemoryCallLog, Outcome, Project, Tool,
};
use tinyinference_llm::message::Message;
use tinyinference_llm::model::ModelProfile;
use tinyinference_llm::tool::ToolCall;
use tinyinference_llm::usage::Usage;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use world::World;

/// One scripted model reply.
#[derive(Clone)]
enum Step {
    Call(&'static str, Value),
    Say(&'static str),
}

/// What one model request offered.
#[derive(Clone, Debug)]
struct Seen {
    child: &'static str,
    tools: Vec<String>,
    model: Option<String>,
    /// User messages the request carried that the duel's monitor wrote.
    nudges: usize,
}

/// A model that plays a queue of steps per child, told apart by system prompt.
struct Scripted {
    queues: Mutex<HashMap<&'static str, VecDeque<Step>>>,
    seen: Mutex<Vec<Seen>>,
}

const MARKERS: [(&str, &str); 6] = [
    ("reader", "You read one arXiv paper"),
    ("formalizer", "You write Lean 4 files"),
    ("reviewer", "You judge whether a Lean declaration"),
    ("refuter", "You look for a counterexample"),
    ("duel-prover", "You are the prover in a duel"),
    ("duel-refuter", "You are the refuter in a duel"),
];

impl Scripted {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            queues: Mutex::new(HashMap::new()),
            seen: Mutex::new(Vec::new()),
        })
    }

    fn queue(&self, child: &'static str, steps: Vec<Step>) {
        self.queues.lock().unwrap().insert(child, steps.into());
    }

    fn seen_for(&self, child: &str) -> Vec<Seen> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|seen| seen.child == child)
            .cloned()
            .collect()
    }
}

#[async_trait::async_trait]
impl ChatModel<()> for Scripted {
    fn profile(&self) -> Option<&ModelProfile> {
        static PROFILE: std::sync::OnceLock<ModelProfile> = std::sync::OnceLock::new();
        Some(PROFILE.get_or_init(|| ModelProfile {
            tool_calling: true,
            model: Some("scripted".to_string()),
            ..ModelProfile::default()
        }))
    }

    async fn invoke(
        &self,
        (): &(),
        request: ModelRequest,
    ) -> openhuman_embed::providers::Result<ModelResponse> {
        let system: String = request
            .messages
            .iter()
            .filter(|m| matches!(m, Message::System(_)))
            .map(Message::text)
            .collect();
        let child = MARKERS
            .iter()
            .find(|(_, marker)| system.contains(marker))
            .map_or("unknown", |(name, _)| *name);
        // OpenHuman forwards steers into a running turn every 50 ms; a duel
        // side answers no faster than a real model would, so a steer queued
        // between two calls is applied before the next one.
        if child.starts_with("duel-") {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
        let nudges = request
            .messages
            .iter()
            .filter(|m| matches!(m, Message::User(_)) && m.text().contains("[Duel monitor]"))
            .count();
        self.seen.lock().unwrap().push(Seen {
            child,
            tools: request.tools.iter().map(|t| t.name.clone()).collect(),
            model: request.model.clone(),
            nudges,
        });
        let step = self
            .queues
            .lock()
            .unwrap()
            .get_mut(child)
            .and_then(VecDeque::pop_front)
            .unwrap_or(Step::Say("done"));
        let usage = Usage {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            ..Usage::default()
        };
        Ok(match step {
            Step::Say(text) => ModelResponse::assistant(text).with_usage(usage),
            Step::Call(name, args) => {
                let mut response = ModelResponse::assistant("").with_usage(usage);
                response.message.tool_calls = vec![ToolCall::new("call-1", name, args)];
                response.with_finish_reason("tool_calls")
            }
        })
    }
}

/// Answers every HTTP request with the JSON the runtime's backend probe wants.
async fn stub_backend() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 8192];
                let _ = socket.read(&mut buffer).await;
                let body = r#"{"success":true,"data":{"id":"embed-test","email":"local@openhuman.local"}}"#;
                let reply = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(reply.as_bytes()).await;
            });
        }
    });
    url
}

fn offline_config() -> RuntimeConfig {
    let mut config = RuntimeConfig::default();
    config.local_ai.runtime_enabled = false;
    config.memory.conversations.enabled = false;
    config.agent.session_dual_write = false;
    config.agent.session_shadow_reads = false;
    config.default_temperature = 0.0;
    config
}

#[derive(Default)]
struct Events(Mutex<Vec<String>>);

impl Progress for Events {
    fn tool_started(&self, parent: &str, tool: &str) {
        self.0.lock().unwrap().push(format!("{parent} -> {tool}"));
    }
    fn tool_ended(&self, _: &str, _: &str, _: Outcome) {}
}

fn ctx(project: &Project) -> CallContext {
    CallContext::new(project.clone(), "e2e").with_budget(Budget {
        wall_time_ms: 3_600_000,
        ..Budget::agentic(100, 5_000_000)
    })
}

/// The tools a request offered, less `OpenHuman`'s own `juice_*` helpers for
/// reading back a large tool output the runtime stored: they reach no
/// supermath primitive and no file, so they are not part of a child's scope.
fn scope_of(seen: &Seen) -> Vec<String> {
    let mut tools: Vec<String> = seen
        .tools
        .iter()
        .filter(|name| !name.starts_with("juice_"))
        .cloned()
        .collect();
    tools.sort();
    tools
}

/// The sorted primitives of `kind`.
fn primitives(kind: supermath_subagents::ChildKind) -> Vec<String> {
    let mut names: Vec<String> = kind
        .spec()
        .primitives
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    names.sort();
    names
}

const PATH: &str = "lean/Supermath/Main.lean";
const SORRY_FILE: &str = "import Mathlib\n\ntheorem main_thm : 1 + 1 = 2 := by\n  sorry\n";

#[test]
fn each_child_runs_as_a_scoped_openhuman_agent_and_the_tool_reports_what_it_verified() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(openhuman_embed::process::AGENT_WORKER_STACK_BYTES)
        .max_blocking_threads(openhuman_embed::process::MAX_BLOCKING_THREADS)
        .build()
        .unwrap();
    runtime.block_on(async { tokio::spawn(scenarios()).await.unwrap() });
}

/// The runtime, the scripted model, the fake world, and the five tools.
struct Rig {
    model: Arc<Scripted>,
    world: World,
    project: Project,
    events: Arc<Events>,
    tools: Vec<Arc<dyn Tool>>,
    _host: Runtime,
    _scratch: tempfile::TempDir,
}

impl Rig {
    async fn start() -> Self {
        let backend = stub_backend().await;
        let scratch = tempfile::tempdir().unwrap();
        // The build future is large; boxing it keeps it off the caller's stack.
        let host = Box::pin(
            Runtime::builder()
                .config(offline_config())
                .workspace(Workspace::Dir(scratch.path().to_path_buf()))
                .backend_url(backend)
                .build(),
        )
        .await
        .unwrap();
        let model = Scripted::new();
        let events = Arc::new(Events::default());
        let world = World::new();
        let project = Project::new("demo", "ws:test/project:demo").unwrap();
        let config = SubagentsConfig {
            models: Models {
                reader: "reader-rung".into(),
                formalizer: "formalizer-rung".into(),
                reviewer: "reviewer-rung".into(),
                duel: "duel-rung".into(),
            },
            provider: Some(Provider::custom(model.clone())),
            progress: Some(events.clone()),
            duel: DuelSettings {
                drift_calls: 2,
                max_rounds: 2,
                digest_every: 8,
            },
        };
        let built = Subagents::new(
            &host,
            &world.registry(),
            Arc::new(InMemoryCallLog::new()),
            &project,
            &config,
        )
        .unwrap();
        assert_eq!(built.tools().len(), 5, "{:?}", built.unavailable());
        Self {
            model,
            world,
            project,
            events,
            tools: built.tools().to_vec(),
            _host: host,
            _scratch: scratch,
        }
    }

    async fn call(&self, tool: &str, args: Value) -> Envelope {
        let tool = self.tools.iter().find(|t| t.name() == tool).unwrap();
        tool.call(&ctx(&self.project), args).await
    }
}

async fn scenarios() {
    let rig = Rig::start().await;
    reader_records_through_its_own_ledger_calls(&rig).await;
    formalizer_claims_success_but_lean_says_failed(&rig).await;
    a_turn_that_outruns_its_iterations_is_cut_off(&rig).await;
    reviewer_sees_signatures_only(&rig).await;
    refuter_witness_is_a_lead_until_lean_confirms(&rig).await;
    a_drifting_duel_side_is_nudged_then_cut_through_openhuman(&rig).await;
    the_duel_prover_wins_and_the_refuter_is_stopped(&rig).await;
    progress_and_usage_are_reported(&rig).await;
}

async fn reader_records_through_its_own_ledger_calls(rig: &Rig) {
    rig.world.add_paper("2012.04125", &[]);
    let source = json!({"arxiv": {"id": "2012.04125", "version": "v2", "label": "thm:main"}});
    rig.model.queue(
        "reader",
        vec![
            Step::Call(
                "ledger_record",
                json!({"kind": "fact", "statement": "Def 1", "source": source}),
            ),
            Step::Call(
                "ledger_record",
                json!({"kind": "lead", "statement": "Open", "source": source}),
            ),
            Step::Say("The paper proves a bound."),
        ],
    );
    let digest = rig
        .call("agent_digest_paper", json!({"arxiv_id": "2012.04125"}))
        .await;
    assert_eq!(digest.outcome, Outcome::Ok, "{digest:?}");
    assert_eq!(digest.data["count"], 2);
    let usage = digest.usage.clone().unwrap();
    assert_eq!(usage.model, "reader-rung");
    assert_eq!(usage.tool_calls, 2);
    assert_eq!(usage.turns, 3);
    assert_eq!(usage.input_tokens, 30);

    let seen = rig.model.seen_for("reader");
    assert_eq!(seen.len(), 3);
    assert_eq!(
        scope_of(&seen[0]),
        primitives(supermath_subagents::ChildKind::Reader),
        "the reader is offered exactly its primitives"
    );
    assert_eq!(seen[0].model.as_deref(), Some("reader-rung"));
}

async fn formalizer_claims_success_but_lean_says_failed(rig: &Rig) {
    rig.world.set_verdict(PATH, "failed");
    rig.model.queue(
        "formalizer",
        vec![
            Step::Call("file_write", json!({"path": PATH, "content": SORRY_FILE})),
            Step::Call("lean_check", json!({"path": PATH})),
            Step::Say("NOTE: lean_check passed with no errors."),
        ],
    );
    let formalized = rig
        .call(
            "agent_formalize",
            json!({"source": {"statement": "one plus one is two"}, "path": PATH}),
        )
        .await;
    assert_eq!(formalized.outcome, Outcome::Ok, "{formalized:?}");
    assert_eq!(formalized.data["verdict"], "failed");
    assert_eq!(formalized.data["recorded"], json!([]));
    assert_eq!(formalized.usage.unwrap().model, "formalizer-rung");
    assert_eq!(
        scope_of(&rig.model.seen_for("formalizer")[0]),
        primitives(supermath_subagents::ChildKind::Formalizer)
    );
    assert!(rig.world.records().iter().all(|r| r["kind"] != "result"));
}

async fn a_turn_that_outruns_its_iterations_is_cut_off(rig: &Rig) {
    rig.model.queue(
        "formalizer",
        (0..6).map(|_| Step::Call("file_list", json!({}))).collect(),
    );
    let before = rig.model.seen_for("formalizer").len();
    let cut = rig
        .call(
            "agent_formalize",
            json!({"source": {"statement": "x"}, "path": "lean/Supermath/Other.lean",
                   "budget": {"max_iterations": 2}}),
        )
        .await;
    assert_eq!(cut.outcome, Outcome::Unknown, "{cut:?}");
    assert!(
        cut.unknowns
            .join(" ")
            .contains("ran out of budget (iterations)"),
        "{cut:?}"
    );
    let used = rig.model.seen_for("formalizer").len() - before;
    assert!(
        used <= 3,
        "the stop hook ended the turn after {used} model calls"
    );
}

async fn reviewer_sees_signatures_only(rig: &Rig) {
    // The file does not check in Lean, so the reviewer's "accepted" is withheld.
    rig.world.add_decl("Main.main_thm", Some(PATH));
    let fact = rig
        .world
        .add_claim(json!({"kind": "fact", "statement": "1 + 1 = 2"}));
    let claim = rig.world.add_claim(json!({
        "kind": "result", "statement": "formal",
        "lean": {"path": PATH, "declaration": "Main.main_thm", "file_sha256": "x"},
        "uses": [fact],
    }));
    rig.model.queue(
        "reviewer",
        vec![
            Step::Call("lean_decl", json!({"name": "Main.main_thm"})),
            Step::Say("VERDICT: accepted"),
        ],
    );
    let reviewed = rig
        .call("agent_review_alignment", json!({"claim_id": claim}))
        .await;
    assert_eq!(reviewed.data["alignment"], "not_reviewed", "{reviewed:?}");
    assert_eq!(reviewed.data["lean_verdict"], "failed");
    let seen = rig.model.seen_for("reviewer");
    assert_eq!(
        scope_of(&seen[0]),
        primitives(supermath_subagents::ChildKind::Reviewer)
    );
    for forbidden in [
        "file_read",
        "file_list",
        "lean_goals",
        "lean_check",
        "file_write",
    ] {
        assert!(!seen[0].tools.iter().any(|t| t == forbidden), "{forbidden}");
    }
    assert_eq!(seen[0].model.as_deref(), Some("reviewer-rung"));
}

async fn refuter_witness_is_a_lead_until_lean_confirms(rig: &Rig) {
    let claim = rig
        .world
        .add_claim(json!({"kind": "result", "statement": "P holds for every n"}));
    rig.world.external_write(
        "lean/Supermath/Witnesses/Notp.lean",
        "import Mathlib\n\ntheorem notp_witness : ∃ n, ¬ P n := ⟨7, by decide⟩\n",
    );
    rig.world.override_tool(
        "lean_search_counterexample",
        // The shape `lean_search_counterexample` answers a found witness in.
        Envelope::ok(json!({
            "found": true,
            "witness": 7,
            "frame": {"predicate": "P", "from": 0, "to": 100, "covered_to": 7, "domain": "nat"},
            "witness_file": "lean/Supermath/Witnesses/Notp.lean",
            "theorem": "notp_witness",
            "evidence_origin": "collected",
            "verdict": "checked",
            "provenance": {"path": "lean/Supermath/Witnesses/Notp.lean", "file_sha256": "x"},
            "errors": [],
        })),
    );
    rig.model.queue(
        "refuter",
        vec![
            Step::Call(
                "lean_search_counterexample",
                json!({"path": PATH, "predicate": "P", "range": {"from": 0, "to": 100}}),
            ),
            Step::Say("REPAIRED: none"),
        ],
    );
    let refuted = rig.call("agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(refuted.outcome, Outcome::Ok, "{refuted:?}");
    assert_eq!(refuted.data["counterexample"]["verdict"], "checked");
    assert!(
        rig.world
            .records()
            .iter()
            .any(|r| r["kind"] == "refutation")
    );

    // A child cannot reach a tool outside its scope even if the model asks.
    rig.model.queue(
        "refuter",
        vec![
            Step::Call(
                "ledger_record",
                json!({"kind": "fact", "statement": "x", "source": {"supplied": "x"}}),
            ),
            Step::Say("REPAIRED: none"),
        ],
    );
    let before = rig.world.records().len();
    rig.call("agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(
        rig.world.records().len(),
        before,
        "the refuter cannot record; only the tool does"
    );
}

async fn progress_and_usage_are_reported(rig: &Rig) {
    let seen = rig.events.0.lock().unwrap().clone();
    assert!(
        seen.contains(&"agent_digest_paper -> ledger_record".to_owned()),
        "{seen:?}"
    );
    assert!(seen.contains(&"agent_formalize -> lean_check".to_owned()));
    // A bad call still carries usage.
    let bad = rig.call("agent_refute", json!({"claim_id": 1})).await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
    assert_eq!(bad.usage.unwrap().model, "formalizer-rung");
}

/// Adds an accepted claim with its Lean file and returns its id and the
/// duel's directory.
fn duel_claim(rig: &Rig) -> (String, String) {
    let file = "lean/Supermath/Duel0.lean";
    let text = "import Mathlib\n\ntheorem duel0 : ∀ n : ℕ, n = n := by\n  sorry\n";
    rig.world.external_write(file, text);
    let id = rig.world.add_claim(json!({
        "kind": "fact",
        "statement": "Every natural number equals itself.",
        "lean": {"path": file, "declaration": "duel0", "file_sha256": world::digest(text)},
        "alignment": "accepted",
    }));
    let dir = format!("lean/Supermath/Duel/C{}", id.trim_start_matches("clm_"));
    (id, dir)
}

async fn a_drifting_duel_side_is_nudged_then_cut_through_openhuman(rig: &Rig) {
    let (id, _) = duel_claim(rig);
    // drift_calls is 2: nudges after calls 2 and 4 arrive as OpenHuman
    // steers, and call 6 has the turn cancelled. The refuter only talks.
    rig.model.queue(
        "duel-prover",
        (0..6)
            .map(|n| Step::Call("file_list", json!({"page": n})))
            .chain([Step::Say("STATUS: lost"), Step::Say("STATUS: still lost")])
            .collect(),
    );
    rig.model.queue(
        "duel-refuter",
        vec![Step::Say("STATUS: nothing"), Step::Say("STATUS: nothing")],
    );
    let answer = rig.call("agent_settle", json!({"claim_id": id})).await;
    assert_eq!(answer.outcome, Outcome::Unknown, "{answer:?}");
    assert_eq!(
        answer.data["sides"]["prover"]["ended"][0], "drift",
        "{answer:?}"
    );
    // The cancelled turn still names its session: the tool minted it before
    // the turn started.
    let prover = &answer.data["sides"]["prover"];
    let sessions = prover["sessions"].as_array().unwrap();
    assert_eq!(Some(sessions.len() as u64), prover["rounds"].as_u64(), "{prover}");
    assert!(
        sessions[0]
            .as_str()
            .unwrap()
            .starts_with("supermath-demo-duel-prover-"),
        "{prover}"
    );
    let seen = rig.model.seen_for("duel-prover");
    assert!(
        seen.iter().any(|s| s.nudges == 2),
        "both nudges reached the model as user messages: {:?}",
        seen.iter().map(|s| s.nudges).collect::<Vec<_>>()
    );
    let mut scope = scope_of(&seen[0]);
    scope.sort();
    assert_eq!(
        scope,
        primitives(supermath_subagents::ChildKind::DuelProver)
    );
    assert_eq!(seen[0].model.as_deref(), Some("duel-rung"));
}

async fn the_duel_prover_wins_and_the_refuter_is_stopped(rig: &Rig) {
    let (id, dir) = duel_claim(rig);
    let main = format!("{dir}/Prove/Main.lean");
    let ns = dir.trim_start_matches("lean/").replace('/', ".");
    let proof = format!(
        "import {ns}.Statement\n\nnamespace {ns}\ntheorem proof : Conjecture := fun n => rfl\nend {ns}\n"
    );
    rig.model.queue(
        "duel-prover",
        vec![
            Step::Call("file_write", json!({"path": main, "content": proof})),
            Step::Call("lean_check", json!({"path": main})),
            Step::Say("STATUS: proved"),
        ],
    );
    rig.model.queue(
        "duel-refuter",
        (0..4)
            .map(|_| Step::Call("duel_board", json!({})))
            .collect(),
    );
    let answer = rig.call("agent_settle", json!({"claim_id": id})).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(answer.data["outcome"], "proved");
    let result = rig
        .world
        .records()
        .into_iter()
        .rfind(|r| r["kind"] == "result")
        .unwrap();
    assert_eq!(
        result["source"]["lean"]["path"],
        format!("{dir}/Proved.lean")
    );
}
