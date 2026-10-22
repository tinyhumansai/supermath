//! `lean_blueprint`: the declaration DAG of the project with each node's
//! status.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, clamp_limit,
};

use super::Shared;
use super::project::lean_files;
use super::wire::{coverage, guard, parse_args, parse_cursor, valid_name};
use crate::declarations::declarations;
use crate::error::{Error, Result};
use crate::verdict::{SORRY_AXIOM, Verdict};

/// The ledger's claim ids, looked up per declaration. The ledger does not
/// depend on this crate; `supermath-mcp` implements this port over it.
pub trait ClaimIndex: Send + Sync {
    /// The id of the ledger claim that rests on `declaration` in `path`, if
    /// one does.
    fn claim_id(&self, project: &str, path: &str, declaration: &str) -> Option<String>;
}

/// Graphs the project's declarations and what each still rests on.
#[derive(Debug)]
pub struct LeanBlueprint {
    shared: Arc<Shared>,
}

impl LeanBlueprint {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    limit: Option<u64>,
    #[serde(default)]
    cursor: Option<String>,
}

/// One declaration in the graph.
struct Node {
    name: String,
    path: String,
    line: usize,
    kind: String,
    status: &'static str,
    debt: bool,
    uses: Vec<String>,
}

/// The names reachable from `start` through `uses`, excluding `start`.
fn reachable(start: &str, nodes: &BTreeMap<String, Node>) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([start.to_owned()]);
    while let Some(name) = queue.pop_front() {
        let Some(node) = nodes.get(&name) else {
            continue;
        };
        for used in &node.uses {
            if used != start && seen.insert(used.clone()) {
                queue.push_back(used.clone());
            }
        }
    }
    seen
}

impl LeanBlueprint {
    /// The project's declarations as nodes, the files with no verdict, and
    /// the files that could not be read with why.
    async fn collect(
        &self,
        ctx: &CallContext,
    ) -> Result<(BTreeMap<String, Node>, Vec<String>, Vec<(String, String)>)> {
        let slug = ctx.project.slug();
        let mut nodes = BTreeMap::new();
        let mut unchecked = Vec::new();
        let found = lean_files(&self.shared.workspace, slug)?;
        let unreadable = found
            .unreadable
            .into_iter()
            .map(|f| (f.path.to_string(), f.reason))
            .collect();
        for file in found.ordered {
            let outcome = match self
                .shared
                .checker
                .check_with(
                    slug,
                    &file.path,
                    None,
                    true,
                    self.shared.config.check_timeout_ms,
                )
                .await
            {
                Ok(outcome) => outcome,
                Err(e @ Error::Unavailable { .. }) => return Err(e),
                Err(_) => {
                    unchecked.push(file.path.to_string());
                    continue;
                }
            };
            let report = outcome.report;
            for decl in declarations(&file.text) {
                let axioms = report.provenance.axioms.get(&decl.name);
                let status = match report.verdict {
                    Verdict::Failed => "failed",
                    Verdict::Unknown => "unknown",
                    _ => match axioms {
                        Some(list) if list.iter().any(|a| a == SORRY_AXIOM) => "checked_with_sorry",
                        None if decl.has_sorry => "checked_with_sorry",
                        _ => "checked",
                    },
                };
                if report.verdict == Verdict::Unknown {
                    unchecked.push(file.path.to_string());
                }
                nodes.insert(
                    decl.name.clone(),
                    Node {
                        uses: report.uses.get(&decl.name).cloned().unwrap_or_default(),
                        debt: report.provenance.debt.contains(&decl.name),
                        name: decl.name,
                        path: file.path.to_string(),
                        line: decl.line,
                        kind: decl.kind,
                        status,
                    },
                );
            }
        }
        let known: BTreeSet<String> = nodes.keys().cloned().collect();
        for node in nodes.values_mut() {
            node.uses
                .retain(|used| used != &node.name && known.contains(used));
            node.uses.sort();
            node.uses.dedup();
        }
        unchecked.sort();
        unchecked.dedup();
        Ok((nodes, unchecked, unreadable))
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let offset = parse_cursor(args.cursor.as_deref())?;
        let limit = clamp_limit(args.limit).min(ctx.budget.rows.max(1));
        let root = args.root.as_deref().map(str::trim);
        if let Some(root) = root
            && !valid_name(root)
        {
            return Err(Error::InvalidInput(format!(
                "{root:?} is not a declaration name"
            )));
        }
        let (nodes, unchecked, unreadable) = self.collect(ctx).await?;
        let wanted: Option<BTreeSet<String>> = match root {
            Some(root) => {
                if !nodes.contains_key(root) {
                    return Err(Error::NotFound(format!(
                        "{root} is not declared in the project"
                    )));
                }
                let mut set = reachable(root, &nodes);
                set.insert(root.to_owned());
                Some(set)
            }
            None => None,
        };
        let mut order: Vec<&Node> = nodes
            .values()
            .filter(|n| wanted.as_ref().is_none_or(|w| w.contains(&n.name)))
            .collect();
        order.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
        let total = order.len();
        let slug = ctx.project.slug();
        let rows: Vec<Value> = order
            .iter()
            .skip(offset)
            .take(limit)
            .map(|node| {
                let rests_on: Vec<String> = reachable(&node.name, &nodes)
                    .into_iter()
                    .filter(|n| {
                        nodes
                            .get(n)
                            .is_some_and(|m| m.status == "checked_with_sorry")
                    })
                    .collect();
                json!({
                    "name": node.name,
                    "path": node.path,
                    "line": node.line,
                    "kind": node.kind,
                    "status": node.status,
                    "debt": node.debt,
                    "claim_id": self.shared.claims.as_ref()
                        .and_then(|c| c.claim_id(slug, &node.path, &node.name)),
                    "uses": node.uses,
                    "rests_on_sorry": rests_on,
                })
            })
            .collect();
        let mut envelope = Envelope::ok(json!({"nodes": rows, "total": total, "root": root}))
            .with_coverage(coverage(
                format!("{total} declaration(s) of the project"),
                &["Mathlib declarations (edges only join project declarations)"],
            ));
        for path in &unchecked {
            envelope = envelope.with_unknown(format!(
                "{path} has no verdict; its nodes may be missing or marked unknown"
            ));
        }
        for (path, why) in &unreadable {
            envelope = envelope.with_unknown(format!(
                "{path} could not be read ({why}); its declarations are missing"
            ));
        }
        let next = offset + limit;
        Ok(if next < total {
            envelope.with_cursor(next.to_string())
        } else {
            envelope
        })
    }
}

#[async_trait]
impl Tool for LeanBlueprint {
    fn name(&self) -> &'static str {
        "lean_blueprint"
    }

    fn description(&self) -> &'static str {
        "Given the project, returns the dependency graph of its Lean declarations: each node with \
         its status (checked, checked_with_sorry, failed, unknown), its ledger claim id, the \
         declarations it uses, and the sorried declarations it transitively rests on. Pass root \
         to see only what one declaration rests on. Formalise the node most others rest on first."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "root": {"type": "string", "description": "Only this declaration and what it rests on"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 50},
                "cursor": {"type": "string", "description": "From a previous call; continues the list"}
            },
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::BackendUnavailable,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        guard(ctx, self.run(ctx, args)).await
    }
}
