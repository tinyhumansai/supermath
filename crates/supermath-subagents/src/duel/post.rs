//! `duel_post` and `duel_board`: how the two sides talk.
//!
//! A side never writes into its partner's context directly. It posts, the
//! host checks what can be checked (a lemma against Lean, a bound against
//! the side's own exhausted searches), records the post in the ledger, and
//! queues it on the board, which delivers it as part of a digest. Anything
//! the host cannot check is relayed as an unverified lead.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::board::{Board, Bucket};
use super::guard::within;
use super::types::{Item, PostKind, Side};
use crate::tools::{Agentic, parse_args};

/// Most words in a post's `text`.
pub(crate) const POST_WORDS: usize = 80;
/// Most posts one side may make in a duel.
pub(crate) const MAX_POSTS: u32 = 12;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PostArgs {
    kind: String,
    text: String,
    path: Option<String>,
    declaration: Option<String>,
}

const POST_DESCRIPTION: &str = "Sends your partner in the duel one finding. A lemma (path and \
     declaration) is relayed only if lean_check says it is checked; a bound relays the frames your \
     searches covered to the end; an obstruction (prover) or candidate (refuter) is relayed as an \
     unverified lead. At most 80 words and 12 posts.";

const BOARD_DESCRIPTION: &str = "Shows what the duel has settled so far: your goal, your \
     partner's status, every verified lemma and bound, and every unverified lead. Marks your \
     partner's posts as read.";

fn post_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": {"type": "string", "enum": ["lemma", "bound", "obstruction", "candidate"]},
            "text": {"type": "string", "description": "The finding, in at most 80 words."},
            "path": {"type": "string", "description": "For a lemma: the Lean file under your area."},
            "declaration": {"type": "string", "description": "For a lemma: its fully qualified name."}
        },
        "required": ["kind", "text"],
        "additionalProperties": false
    })
}

fn board_schema() -> Value {
    json!({"type": "object", "properties": {}, "additionalProperties": false})
}

/// `duel_post` for one side.
pub(crate) struct DuelPost {
    pub(crate) side: Side,
    pub(crate) board: Arc<Board>,
    pub(crate) core: Arc<Agentic>,
}

fn invalid(message: impl Into<String>) -> Envelope {
    Envelope::error(ErrorCode::InvalidInput, message)
}

fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

impl DuelPost {
    /// Checks a lemma in Lean and relays it as verified.
    async fn lemma(&self, ctx: &CallContext, args: &PostArgs) -> Result<Vec<Item>, Box<Envelope>> {
        let (Some(path), Some(declaration)) = (&args.path, &args.declaration) else {
            return Err(Box::new(invalid(
                "a lemma post needs `path` and `declaration`",
            )));
        };
        let areas = self.board.target().areas(self.side);
        let lean = std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lean"));
        if !lean || !within(path, &areas) {
            return Err(Box::new(invalid(format!(
                "a lemma must be in a Lean file under your area ({})",
                areas.join(" or ")
            ))));
        }
        let verdict = self
            .core
            .lean_verdict_for(ctx, json!({"path": path, "targets": [declaration]}))
            .await;
        let Some(sha) = verdict.file_sha256.filter(|_| verdict.verdict == "checked") else {
            return Err(Box::new(invalid(format!(
                "lean_check says {} for {declaration} in {path}; only a checked lemma is relayed",
                verdict.verdict
            ))));
        };
        let claim = &self.board.target().claim_id;
        let recorded = self
            .core
            .record(
                ctx,
                json!({
                    "kind": "fact",
                    "statement": format!(
                        "Lemma {declaration} in {path}, checked in Lean while settling {claim}: {}",
                        args.text
                    ),
                    "source": {"lean": {"path": path, "declaration": declaration, "file_sha256": sha}},
                }),
            )
            .await;
        if let Ok(entry) = recorded {
            self.board.keep_entry(Bucket::Lemma, entry.to_json());
        }
        Ok(vec![Item {
            from: self.side,
            verified: true,
            line: format!(
                "VERIFIED lemma {declaration} in {path} (sha {}): {}",
                short(&sha),
                args.text
            ),
        }])
    }

    /// Records each frame the side's searches covered since its last bound
    /// post, and relays them as verified.
    async fn bound(&self, ctx: &CallContext, args: &PostArgs) -> Result<Vec<Item>, Box<Envelope>> {
        let frames = self.board.take_unposted_frames(self.side);
        if frames.is_empty() {
            return Err(Box::new(invalid(
                "no search of yours covered a new frame to the end since your last bound post",
            )));
        }
        let claim = &self.board.target().claim_id;
        let mut items = Vec::new();
        for frame in frames {
            let recorded = self
                .core
                .record(
                    ctx,
                    json!({
                        "kind": "bound",
                        "statement": format!("No counterexample to {claim} found in: {}", frame.range),
                        "source": {"tool": {"tool": frame.tool, "call_id": frame.call_id}},
                        "frame": frame.range,
                    }),
                )
                .await;
            if let Ok(entry) = recorded {
                self.board.keep_entry(Bucket::Bound, entry.to_json());
            }
            items.push(Item {
                from: self.side,
                verified: true,
                line: format!(
                    "VERIFIED bound: no counterexample in {} ({})",
                    frame.range, args.text
                ),
            });
        }
        Ok(items)
    }

    /// Records an unchecked hint as a lead and relays it as one.
    async fn lead(&self, ctx: &CallContext, kind: PostKind, args: &PostArgs) -> Vec<Item> {
        let claim = &self.board.target().claim_id;
        let recorded = self
            .core
            .record(
                ctx,
                json!({
                    "kind": "lead",
                    "statement": args.text,
                    "source": {"supplied": format!(
                        "duel_post {} by the {} while settling {claim}",
                        kind.as_str(),
                        self.side.name()
                    )},
                    "rationale": format!(
                        "A {} the {} reported during the duel; not checked.",
                        kind.as_str(),
                        self.side.name()
                    ),
                }),
            )
            .await;
        if let Ok(entry) = recorded {
            self.board.keep_entry(Bucket::Lead, entry.to_json());
        }
        vec![Item {
            from: self.side,
            verified: false,
            line: format!(
                "UNVERIFIED lead ({} from the {}): {}",
                kind.as_str(),
                self.side.name(),
                args.text
            ),
        }]
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Envelope {
        let args: PostArgs = match parse_args(args) {
            Ok(args) => args,
            Err(envelope) => return *envelope,
        };
        let Some(kind) = PostKind::parse(&args.kind) else {
            return invalid("kind must be lemma, bound, obstruction, or candidate");
        };
        if !kind.allowed_for(self.side) {
            return invalid(format!(
                "the {} cannot post a {}",
                self.side.name(),
                kind.as_str()
            ));
        }
        let words = args.text.split_whitespace().count();
        if words == 0 || words > POST_WORDS {
            return invalid(format!(
                "text must hold 1 to {POST_WORDS} words; it holds {words}"
            ));
        }
        if !self.board.take_post_slot(self.side, MAX_POSTS) {
            return Envelope::error(
                ErrorCode::RateLimited,
                format!("you have used all {MAX_POSTS} posts of this duel"),
            );
        }
        let items = match kind {
            PostKind::Lemma => self.lemma(ctx, &args).await,
            PostKind::Bound => self.bound(ctx, &args).await,
            PostKind::Obstruction | PostKind::Candidate => Ok(self.lead(ctx, kind, &args).await),
        };
        match items {
            Ok(items) => {
                let lines: Vec<&str> = items.iter().map(|item| item.line.as_str()).collect();
                let data = json!({"relayed": lines});
                for item in items {
                    self.board.relay(item);
                }
                Envelope::ok(data)
            }
            Err(envelope) => *envelope,
        }
    }
}

#[async_trait]
impl Tool for DuelPost {
    fn name(&self) -> &'static str {
        "duel_post"
    }

    fn description(&self) -> &'static str {
        POST_DESCRIPTION
    }

    fn schema(&self) -> Value {
        post_schema()
    }

    fn exposure(&self) -> Exposure {
        Exposure::Direct
    }

    fn permission(&self) -> Permission {
        Permission::Write
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[ErrorCode::InvalidInput, ErrorCode::RateLimited]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        self.run(ctx, args).await
    }
}

/// `duel_board` for one side.
pub(crate) struct DuelBoard {
    pub(crate) side: Side,
    pub(crate) board: Arc<Board>,
}

#[async_trait]
impl Tool for DuelBoard {
    fn name(&self) -> &'static str {
        "duel_board"
    }

    fn description(&self) -> &'static str {
        BOARD_DESCRIPTION
    }

    fn schema(&self) -> Value {
        board_schema()
    }

    fn exposure(&self) -> Exposure {
        Exposure::Direct
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[ErrorCode::InvalidInput]
    }

    async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
        if args.as_object().is_some_and(|object| !object.is_empty()) {
            return invalid("duel_board takes no arguments");
        }
        Envelope::ok(self.board.view(self.side))
    }
}

/// `duel_post` or `duel_board` outside a duel: same name and schema, so a
/// duel side's agent is built with the tools each of its turns will carry,
/// but every call is refused.
pub(crate) struct Offline(pub(crate) &'static str);

#[async_trait]
impl Tool for Offline {
    fn name(&self) -> &'static str {
        self.0
    }

    fn description(&self) -> &'static str {
        if self.0 == "duel_post" {
            POST_DESCRIPTION
        } else {
            BOARD_DESCRIPTION
        }
    }

    fn schema(&self) -> Value {
        if self.0 == "duel_post" {
            post_schema()
        } else {
            board_schema()
        }
    }

    fn exposure(&self) -> Exposure {
        Exposure::Direct
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[ErrorCode::InvalidInput]
    }

    async fn call(&self, _ctx: &CallContext, _args: Value) -> Envelope {
        invalid(format!("{} works only inside an agent_settle duel", self.0))
    }
}
