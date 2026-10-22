//! Effective status: a claim is no stronger than the weakest claim it uses,
//! and no stronger than its Lean file still allows.
//!
//! A stored `formalised` status holds only for the digest it was earned at.
//! On every read the ledger asks the [`LeanEvidence`](crate::LeanEvidence)
//! port again; a file whose digest moved, or whose check no longer earns
//! `formalised`, stands as `asserted` until it is recorded again.

use std::collections::BTreeMap;

use super::Ledger;
use crate::error::Result;
use crate::types::{Entry, Status};

/// A claim's status after its own Lean file and everything it uses are
/// taken into account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effective {
    /// The effective status.
    pub status: Status,
    /// The claims that limit it, from this claim down to the weakest one it
    /// reaches. Just this claim when nothing it uses is weaker.
    pub chain: Vec<String>,
    /// Why a status was lowered, when it was.
    pub notes: Vec<String>,
}

/// A claim's own status, before its edges.
struct Standing {
    status: Status,
    note: Option<String>,
}

impl Ledger {
    /// The effective status of each of `entries`, in order.
    ///
    /// # Errors
    ///
    /// The memory errors of [`Ledger::get`]. A Lean check that cannot run
    /// lowers the claim to `asserted` with a note; it is not an error.
    pub async fn effective(&self, slug: &str, entries: &[Entry]) -> Result<Vec<Effective>> {
        let (nodes, standings) = self.graph(slug, entries).await?;
        let mut memo = BTreeMap::new();
        Ok(entries
            .iter()
            .map(|e| resolve(&e.id, &nodes, &standings, &mut memo, &mut Vec::new()))
            .collect())
    }

    /// The effective status of one entry.
    ///
    /// # Errors
    ///
    /// The same as [`Ledger::effective`].
    pub(crate) async fn effective_of(&self, slug: &str, entry: &Entry) -> Result<Effective> {
        let (nodes, standings) = self.graph(slug, std::slice::from_ref(entry)).await?;
        Ok(resolve(
            &entry.id,
            &nodes,
            &standings,
            &mut BTreeMap::new(),
            &mut Vec::new(),
        ))
    }

    /// Everything `entries` reach, and each claim's own standing.
    async fn graph(
        &self,
        slug: &str,
        entries: &[Entry],
    ) -> Result<(BTreeMap<String, Entry>, BTreeMap<String, Standing>)> {
        let roots: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
        let cache = entries.iter().map(|e| (e.id.clone(), e.clone())).collect();
        let nodes = self.closure(slug, &roots, cache).await?;
        let mut standings = BTreeMap::new();
        for (id, entry) in &nodes {
            standings.insert(id.clone(), self.standing(entry).await);
        }
        Ok((nodes, standings))
    }

    /// Re-asks Lean about a `formalised` claim; every other status stands.
    async fn standing(&self, entry: &Entry) -> Standing {
        let (Status::Formalised, Some(lean)) = (entry.status, entry.lean.as_ref()) else {
            return Standing {
                status: entry.status,
                note: None,
            };
        };
        let lowered = |why: String| Standing {
            status: Status::Asserted,
            note: Some(format!("{}: {why}", entry.id)),
        };
        match self.evidence.verify(&lean.path, &lean.declaration).await {
            Ok(verdict) if verdict.file_sha256 != lean.file_sha256 => {
                lowered(format!("{} changed since it was checked", lean.path))
            }
            Ok(verdict) if !verdict.earns_formalised(&lean.declaration) => {
                lowered("the file no longer checks cleanly".to_string())
            }
            Ok(_) => Standing {
                status: Status::Formalised,
                note: None,
            },
            Err(error) => lowered(format!("it could not be re-checked ({error})")),
        }
    }
}

/// Resolves `id`'s effective status over `nodes`. A claim not in the
/// ledger stands as `asserted`; a cycle is cut where it closes.
fn resolve(
    id: &str,
    nodes: &BTreeMap<String, Entry>,
    standings: &BTreeMap<String, Standing>,
    memo: &mut BTreeMap<String, Effective>,
    visiting: &mut Vec<String>,
) -> Effective {
    if let Some(done) = memo.get(id) {
        return done.clone();
    }
    let (Some(entry), Some(standing)) = (nodes.get(id), standings.get(id)) else {
        return Effective {
            status: Status::Asserted,
            chain: vec![id.to_string()],
            notes: vec![format!("{id}: not in the ledger")],
        };
    };
    if visiting.iter().any(|v| v == id) {
        return Effective {
            status: Status::Formalised,
            chain: vec![id.to_string()],
            notes: Vec::new(),
        };
    }
    visiting.push(id.to_string());
    let mut best = Effective {
        status: standing.status,
        chain: vec![id.to_string()],
        notes: standing.note.iter().cloned().collect(),
    };
    for used in &entry.uses {
        let below = resolve(used, nodes, standings, memo, visiting);
        if below.status < best.status {
            let mut chain = vec![id.to_string()];
            chain.extend(below.chain);
            best = Effective {
                status: below.status,
                chain,
                notes: below.notes,
            };
        }
    }
    visiting.pop();
    memo.insert(id.to_string(), best.clone());
    best
}
