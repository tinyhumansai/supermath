//! Fake primitives backed by one in-memory world: files, a ledger, papers,
//! and Lean verdicts. Shared by the unit tests and the `OpenHuman` end-to-end
//! test, which includes this file by path, so it depends on `supermath_tool`
//! alone.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Outcome, Permission, Tool, ToolRegistry,
};

/// The primitive names the four children and their verifications use.
pub(crate) const PRIMITIVES: [&str; 22] = [
    "file_edit",
    "file_list",
    "file_read",
    "file_write",
    "ledger_claims",
    "ledger_link",
    "ledger_record",
    "lean_check",
    "lean_decl",
    "lean_goals",
    "lean_search",
    "lean_search_counterexample",
    "lean_try_tactics",
    "lit_citations",
    "lit_extract_statements",
    "lit_get_paper",
    "lit_oeis",
    "py_run",
    "py_search_counterexample",
    "py_solvers",
    "recall_memory",
    "ledger_bound",
];

#[derive(Default)]
struct State {
    files: BTreeMap<String, String>,
    verdicts: BTreeMap<String, String>,
    claims: Vec<Value>,
    papers: BTreeMap<String, Vec<String>>,
    decls: BTreeMap<String, Option<String>>,
    overrides: BTreeMap<String, Envelope>,
    records: Vec<Value>,
    /// Every non-ledger call so far, by call id: the tool and how it ended.
    /// The real ledger accepts a `tool` source only from such a call.
    calls: BTreeMap<String, (String, Outcome)>,
    force_status: Option<String>,
    refuse_records: bool,
}

/// A shared, mutable fake world. Clones see the same state.
#[derive(Clone, Default)]
pub(crate) struct World {
    state: Arc<Mutex<State>>,
}

/// FNV-1a over `text`, hex: stands in for a SHA-256 digest.
pub(crate) fn digest(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

impl World {
    /// An empty world.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn with<T>(&self, f: impl FnOnce(&mut State) -> T) -> T {
        f(&mut self.state.lock().unwrap())
    }

    /// A registry of every primitive, backed by this world.
    pub(crate) fn registry(&self) -> ToolRegistry {
        self.registry_of(&PRIMITIVES)
    }

    /// A registry of the named primitives only.
    pub(crate) fn registry_of(&self, names: &[&'static str]) -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        for name in names {
            registry
                .register(Arc::new(FakeTool {
                    name,
                    world: self.clone(),
                }))
                .unwrap();
        }
        registry
    }

    /// Makes `arxiv_id` a known paper with these statement labels.
    pub(crate) fn add_paper(&self, arxiv_id: &str, labels: &[&str]) {
        self.with(|s| {
            s.papers.insert(
                arxiv_id.into(),
                labels.iter().map(|l| (*l).into()).collect(),
            );
        });
    }

    /// Makes `name` a known Lean declaration, optionally in a project file.
    pub(crate) fn add_decl(&self, name: &str, path: Option<&str>) {
        self.with(|s| s.decls.insert(name.into(), path.map(Into::into)));
    }

    /// Adds a ledger claim and returns its id.
    pub(crate) fn add_claim(&self, mut claim: Value) -> String {
        self.with(|s| {
            let id = format!("clm_{}", s.claims.len() + 1);
            claim["id"] = json!(id);
            s.claims.push(claim);
            id
        })
    }

    /// Forces `lean_check` to answer `verdict` for `path`.
    pub(crate) fn set_verdict(&self, path: &str, verdict: &str) {
        self.with(|s| s.verdicts.insert(path.into(), verdict.into()));
    }

    /// Makes `tool` answer `envelope` regardless of its arguments.
    pub(crate) fn override_tool(&self, tool: &str, envelope: Envelope) {
        self.with(|s| s.overrides.insert(tool.into(), envelope));
    }

    /// Makes `ledger_record` report this status for every entry.
    pub(crate) fn force_status(&self, status: &str) {
        self.with(|s| s.force_status = Some(status.into()));
    }

    /// Makes `ledger_record` refuse every call.
    pub(crate) fn refuse_records(&self) {
        self.with(|s| s.refuse_records = true);
    }

    /// Removes a ledger claim, as if the ledger never held it.
    pub(crate) fn forget_claim(&self, id: &str) {
        self.with(|s| s.claims.retain(|c| c["id"] != id));
    }

    /// Replaces a file as a different writer would, outside any tool call.
    pub(crate) fn external_write(&self, path: &str, text: &str) {
        self.with(|s| s.files.insert(path.into(), text.into()));
    }

    /// A file's text.
    pub(crate) fn file(&self, path: &str) -> Option<String> {
        self.with(|s| s.files.get(path).cloned())
    }

    /// The arguments of every accepted `ledger_record` call, in order.
    pub(crate) fn records(&self) -> Vec<Value> {
        self.with(|s| s.records.clone())
    }

    fn handle(&self, name: &str, call_id: &str, args: &Value) -> Envelope {
        let envelope = self.answer(name, args);
        if !name.starts_with("ledger_") {
            self.with(|s| {
                s.calls
                    .insert(call_id.to_owned(), (name.to_owned(), envelope.outcome));
            });
        }
        envelope
    }

    fn answer(&self, name: &str, args: &Value) -> Envelope {
        if let Err(envelope) = check_args(name, args) {
            return *envelope;
        }
        if let Some(envelope) = self.with(|s| s.overrides.get(name).cloned()) {
            return envelope;
        }
        match name {
            "file_write" => self.file_write(args),
            "file_edit" => self.file_edit(args),
            "file_read" => self.file_read(args),
            "file_list" => {
                let files: Vec<String> = self.with(|s| s.files.keys().cloned().collect());
                Envelope::ok(json!({"total": files.len(), "files": files}))
            }
            "lean_check" => self.lean_check(args),
            "ledger_record" => self.ledger_record(args),
            "ledger_claims" => self.ledger_claims(args),
            "lit_get_paper" => self.lit_get_paper(args),
            "lit_extract_statements" => self.lit_extract_statements(args),
            "lean_decl" => {
                let decl = args["name"].as_str().unwrap_or_default();
                match self.with(|s| s.decls.get(decl).cloned()) {
                    Some(path) => {
                        let mut data = json!({
                            "name": decl, "kind": "theorem", "signature": "…", "doc": null,
                            "module": "Main",
                        });
                        if let Some(path) = path {
                            data["path"] = json!(path);
                            data["line"] = json!(1);
                        }
                        Envelope::ok(data)
                    }
                    None => Envelope::error(ErrorCode::NotFound, "no such declaration"),
                }
            }
            _ => Envelope::ok(json!({})),
        }
    }

    fn file_write(&self, args: &Value) -> Envelope {
        let (Some(path), Some(content)) = (args["path"].as_str(), args["content"].as_str()) else {
            return Envelope::error(ErrorCode::InvalidInput, "path and content are required");
        };
        self.with(|s| {
            let current = s.files.get(path).map(|text| digest(text));
            if current.as_deref() != args["if_sha256"].as_str() {
                return Envelope::error(ErrorCode::Conflict, "if_sha256 does not match");
            }
            s.files.insert(path.into(), content.into());
            Envelope::ok(
                json!({"path": path, "sha256": digest(content), "lines": content.lines().count()}),
            )
        })
    }

    fn file_edit(&self, args: &Value) -> Envelope {
        let path = args["path"].as_str().unwrap_or_default();
        let edits = args["edits"].as_array().cloned().unwrap_or_default();
        self.with(|s| {
            let Some(text) = s.files.get_mut(path) else {
                return Envelope::error(ErrorCode::NotFound, "no such file");
            };
            if args["if_sha256"].as_str() != Some(digest(text).as_str()) {
                return Envelope::error(ErrorCode::Conflict, "if_sha256 does not match");
            }
            let mut edited = text.clone();
            for edit in &edits {
                let (Some(old), Some(new)) = (edit["old"].as_str(), edit["new"].as_str()) else {
                    return Envelope::error(ErrorCode::InvalidInput, "old and new are required");
                };
                if edited.matches(old).count() != 1 {
                    return Envelope::error(ErrorCode::InvalidInput, "old must occur exactly once");
                }
                edited = edited.replacen(old, new, 1);
            }
            *text = edited;
            Envelope::ok(json!({"path": path, "sha256": digest(text), "changed": []}))
        })
    }

    fn file_read(&self, args: &Value) -> Envelope {
        let path = args["path"].as_str().unwrap_or_default();
        let Some(text) = self.file(path) else {
            return Envelope::error(ErrorCode::NotFound, "no such file");
        };
        let lines: Vec<&str> = text.lines().collect();
        let starts: Vec<usize> = (0..lines.len())
            .filter(|&i| {
                ["theorem ", "lemma ", "def "]
                    .iter()
                    .any(|kw| lines[i].starts_with(kw))
            })
            .collect();
        let outline: Vec<Value> = starts
            .iter()
            .enumerate()
            .map(|(n, &start)| {
                let end = starts.get(n + 1).copied().unwrap_or(lines.len());
                let mut words = lines[start].split_whitespace();
                let kind = words.next().unwrap_or_default();
                let name = words
                    .next()
                    .unwrap_or_default()
                    .split(':')
                    .next()
                    .unwrap_or_default();
                json!({
                    "kind": kind, "name": name, "start_line": start + 1, "end_line": end,
                    "has_sorry": lines[start..end].iter().any(|l| l.contains("sorry")),
                })
            })
            .collect();
        Envelope::ok(json!({
            "path": path, "language": "lean", "sha256": digest(&text), "lines": lines.len(),
            "outline": outline,
        }))
    }

    fn lean_check(&self, args: &Value) -> Envelope {
        let path = args["path"].as_str().unwrap_or_default();
        let Some(text) = self.file(path) else {
            return Envelope::error(ErrorCode::NotFound, "no such file");
        };
        let verdict = self
            .with(|s| s.verdicts.get(path).cloned())
            .unwrap_or_else(|| {
                if text.contains("ERROR") {
                    "failed"
                } else if text.contains("sorry") {
                    "checked_with_sorry"
                } else {
                    "checked"
                }
                .into()
            });
        let errors = if verdict == "failed" {
            vec![json!({"line": 1, "column": 0, "message": "error"})]
        } else {
            vec![]
        };
        let sorries = if verdict == "checked_with_sorry" {
            vec![json!({"line": 1, "column": 0, "goal": "⊢ True"})]
        } else {
            vec![]
        };
        let targets: Vec<String> = match args.get("targets").and_then(Value::as_array) {
            Some(given) => given
                .iter()
                .filter_map(|t| t.as_str().map(str::to_owned))
                .collect(),
            None => declared_names(&text),
        };
        let data = json!({
            "verdict": verdict,
            "provenance": {
                "toolchain": "leanprover/lean4:v4.20.0", "mathlib": "abc123", "path": path,
                "file_sha256": digest(&text), "imports_sha256": digest(""), "elapsed_ms": 1,
                "axioms": {}, "debt": [],
            },
            "targets": targets,
            "errors": errors,
            "error_count": errors.len(),
            "sorries": sorries,
        });
        if verdict == "unknown" {
            Envelope::unknown("lean_check timed out").with_data(data)
        } else {
            Envelope::ok(data)
        }
    }

    fn ledger_record(&self, args: &Value) -> Envelope {
        let refuse = self.with(|s| s.refuse_records);
        if refuse {
            return Envelope::error(ErrorCode::Forbidden, "the ledger refuses this entry");
        }
        if args["statement"].as_str().is_none_or(str::is_empty) || args["kind"].is_null() {
            return Envelope::error(ErrorCode::InvalidInput, "kind and statement are required");
        }
        let mut earned = "asserted";
        if args["source"].get("arxiv").is_some() {
            earned = "cited";
        }
        if let Some(source) = args["source"].get("tool") {
            let call_id = source["call_id"].as_str().unwrap_or_default();
            let tool = source["tool"].as_str().unwrap_or_default();
            match self.with(|s| s.calls.get(call_id).cloned()) {
                None => {
                    return Envelope::error(
                        ErrorCode::Forbidden,
                        format!("no call {call_id} was made in this process"),
                    );
                }
                Some((called, outcome)) if called != tool || outcome != Outcome::Ok => {
                    return Envelope::error(
                        ErrorCode::Forbidden,
                        format!(
                            "call {call_id} cannot back this claim: it was not an ok call to {tool}"
                        ),
                    );
                }
                Some(_) => {
                    if ["calc_", "py_", "lean_"]
                        .iter()
                        .any(|p| tool.starts_with(p))
                    {
                        earned = "checked";
                    }
                }
            }
        }
        let status = self
            .with(|s| s.force_status.clone())
            .unwrap_or_else(|| earned.into());
        let id = self.add_claim(json!({
            "kind": args["kind"],
            "statement": args["statement"],
            "source": args["source"],
            "lean": args["source"].get("lean"),
            "uses": args.get("uses").cloned().unwrap_or(json!([])),
            "status": status,
            "alignment": "not_reviewed",
        }));
        self.with(|s| s.records.push(args.clone()));
        Envelope::ok(json!({
            "id": id, "kind": args["kind"], "status": status,
            "alignment": "not_reviewed", "created": true,
        }))
    }

    fn ledger_claims(&self, args: &Value) -> Envelope {
        let id = args["id"].as_str().unwrap_or_default();
        match self.with(|s| s.claims.iter().find(|c| c["id"] == id).cloned()) {
            Some(claim) => Envelope::ok(json!({"claim": claim, "effective_status": "asserted"})),
            None => Envelope::error(ErrorCode::NotFound, "no such claim"),
        }
    }

    fn lit_extract_statements(&self, args: &Value) -> Envelope {
        let id = args["arxiv_id"].as_str().unwrap_or_default();
        let Some(labels) = self.with(|s| s.papers.get(id).cloned()) else {
            return Envelope::error(ErrorCode::NotFound, "no such paper");
        };
        let start = args["cursor"]
            .as_str()
            .and_then(|c| c.parse::<usize>().ok())
            .unwrap_or(0);
        let limit = usize::try_from(args["limit"].as_u64().unwrap_or(10)).unwrap_or(10);
        let rows: Vec<Value> = labels
            .iter()
            .enumerate()
            .skip(start)
            .take(limit)
            .map(|(n, label)| {
                json!({
                    "label": label, "kind": "theorem", "number": format!("{}", n + 1),
                    "section": "1", "statement": "…", "refs": [],
                })
            })
            .collect();
        let shown = start + rows.len();
        let envelope = Envelope::ok(json!({
            "locator": {"arxiv_id": id, "version": "v2"},
            "total": labels.len(),
            "statements": rows,
        }));
        if shown < labels.len() {
            envelope.with_cursor(shown.to_string())
        } else {
            envelope
        }
    }

    fn lit_get_paper(&self, args: &Value) -> Envelope {
        let id = args["arxiv_id"].as_str().unwrap_or_default();
        if self.with(|s| s.papers.contains_key(id)) {
            Envelope::ok(json!({"arxiv_id": id, "version": "v2", "title": "A paper"}))
        } else {
            Envelope::error(ErrorCode::NotFound, "no such paper")
        }
    }
}

/// The fully qualified names of the theorems, lemmas, and defs a file
/// declares, in file order, as `lean_check` reports its default targets.
fn declared_names(text: &str) -> Vec<String> {
    let mut scopes: Vec<String> = Vec::new();
    let mut names = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("namespace") => scopes.push(words.next().unwrap_or_default().to_owned()),
            Some("end") => {
                scopes.pop();
            }
            Some("theorem" | "lemma" | "def") => {
                let name = words
                    .next()
                    .unwrap_or_default()
                    .split(':')
                    .next()
                    .unwrap_or_default();
                let mut parts: Vec<&str> = scopes.iter().map(String::as_str).collect();
                parts.push(name);
                names.push(parts.join("."));
            }
            _ => {}
        }
    }
    names
}

/// The real schema of `name`, as far as this crate sends it: the fields it
/// accepts and the ones it requires. Every real primitive has
/// `additionalProperties: false`, so a stray or missing field is
/// `INVALID_INPUT` here exactly as it is there.
fn shape(name: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match name {
        "file_write" => (&["path", "content", "if_sha256"], &["path", "content"]),
        "file_edit" => (
            &["path", "edits", "if_sha256"],
            &["path", "edits", "if_sha256"],
        ),
        "file_read" => (&["path", "start_line", "end_line", "cursor"], &["path"]),
        "lean_check" => (&["path", "targets"], &["path"]),
        "lean_decl" => (&["name"], &["name"]),
        "lean_search_counterexample" => (
            &["path", "predicate", "range"],
            &["path", "predicate", "range"],
        ),
        "py_search_counterexample" => (
            &["path", "predicate", "frame", "seed", "timeout_s"],
            &["path", "predicate", "frame"],
        ),
        "lit_get_paper" => (&["arxiv_id", "section", "cursor"], &["arxiv_id"]),
        "lit_extract_statements" => (&["arxiv_id", "limit", "cursor"], &["arxiv_id"]),
        "ledger_claims" => (
            &[
                "id",
                "kind",
                "min_status",
                "alignment",
                "query",
                "limit",
                "cursor",
            ],
            &[],
        ),
        "ledger_record" => (
            &[
                "kind",
                "statement",
                "source",
                "uses",
                "rationale",
                "reason",
                "refutes",
                "frame",
                "status",
                "evidence_origin",
            ],
            &["kind", "statement", "source"],
        ),
        _ => return None,
    })
}

/// Refuses arguments the real tool's schema would refuse.
fn check_args(name: &str, args: &Value) -> Result<(), Box<Envelope>> {
    let Some((allowed, required)) = shape(name) else {
        return Ok(());
    };
    let bad = |message: String| Err(Box::new(Envelope::error(ErrorCode::InvalidInput, message)));
    let Some(object) = args.as_object() else {
        return bad(format!("{name}: arguments must be an object"));
    };
    if let Some(stray) = object.keys().find(|k| !allowed.contains(&k.as_str())) {
        return bad(format!("{name}: unknown field `{stray}`"));
    }
    if let Some(missing) = required.iter().find(|k| !object.contains_key(**k)) {
        return bad(format!("{name}: missing field `{missing}`"));
    }
    if name == "ledger_record" {
        check_source(&args["source"])?;
        check_enum(
            "kind",
            &args["kind"],
            &[
                "fact",
                "result",
                "intake",
                "bound",
                "lead",
                "dead_end",
                "refutation",
            ],
        )?;
        if args["source"].get("tool").is_some()
            && args["frame"].as_str().is_none_or(|f| f.trim().is_empty())
        {
            return bad("ledger_record: a tool source needs a frame".into());
        }
    }
    Ok(())
}

fn check_enum(field: &str, value: &Value, allowed: &[&str]) -> Result<(), Box<Envelope>> {
    if value.as_str().is_some_and(|v| allowed.contains(&v)) {
        Ok(())
    } else {
        Err(Box::new(Envelope::error(
            ErrorCode::InvalidInput,
            format!("ledger_record: `{field}` must be one of {allowed:?}"),
        )))
    }
}

/// The five `source` shapes of `ledger_record`, each with exactly its
/// fields.
fn check_source(source: &Value) -> Result<(), Box<Envelope>> {
    let shapes: [(&str, &[&str], &[&str]); 4] = [
        (
            "arxiv",
            &["id", "version", "label", "section"],
            &["id", "version"],
        ),
        (
            "lean",
            &["path", "declaration", "file_sha256"],
            &["path", "declaration", "file_sha256"],
        ),
        (
            "tool",
            &["tool", "call_id", "args_digest"],
            &["tool", "call_id"],
        ),
        (
            "web",
            &["url", "title", "class", "retrieved_at"],
            &["url", "title", "class", "retrieved_at"],
        ),
    ];
    let bad = |message: String| Err(Box::new(Envelope::error(ErrorCode::InvalidInput, message)));
    let Some(object) = source.as_object().filter(|o| o.len() == 1) else {
        return bad(
            "ledger_record: source is exactly one of arxiv, lean, tool, web, supplied".into(),
        );
    };
    if object.get("supplied").is_some_and(Value::is_string) {
        return Ok(());
    }
    for (key, allowed, required) in shapes {
        let Some(inner) = object.get(key) else {
            continue;
        };
        let Some(fields) = inner.as_object() else {
            return bad(format!("ledger_record: source.{key} must be an object"));
        };
        let strings = fields.values().all(Value::is_string);
        let known = fields.keys().all(|k| allowed.contains(&k.as_str()));
        let complete = required.iter().all(|k| fields.contains_key(*k));
        return if strings && known && complete {
            Ok(())
        } else {
            bad(format!(
                "ledger_record: source.{key} needs {required:?} and nothing but {allowed:?}"
            ))
        };
    }
    bad("ledger_record: source is exactly one of arxiv, lean, tool, web, supplied".into())
}

/// One primitive, answered by the world.
struct FakeTool {
    name: &'static str,
    world: World,
}

#[async_trait]
impl Tool for FakeTool {
    fn name(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        "A fake primitive backed by an in-memory world."
    }
    fn schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "additionalProperties": true})
    }
    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }
    fn permission(&self) -> Permission {
        if matches!(
            self.name,
            "file_write" | "file_edit" | "ledger_record" | "ledger_link"
        ) {
            Permission::Write
        } else {
            Permission::Read
        }
    }
    fn kind(&self) -> Kind {
        Kind::Primitive
    }
    fn errors(&self) -> &'static [ErrorCode] {
        &[]
    }
    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        self.world.handle(self.name, &ctx.call_id, &args)
    }
}
