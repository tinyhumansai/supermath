//! [`Ledger`]: claims in `CortexDB`, with statuses earned only on evidence.
//!
//! The ledger writes each [`Entry`] as one experience whose text is the
//! entry's JSON, labelled `kind:`, `status:`, and `claim:`. An entry is never
//! edited: a change writes the next revision, and readers take the highest.
//!
//! # Reading
//!
//! `CortexDB` recall is by similarity and does not filter by label exactly
//! (`docs/specs/toolkit/ledger/README.md`, open question 1), so every read
//! recalls a window of the project's notes, ranked by similarity to a query,
//! parses each as an [`Entry`], skips notes that are not entries, and filters
//! client-side. The window is [`DEFAULT_WINDOW`] notes; a ledger larger than
//! that can hide entries from a read, which `ledger_claims` and
//! `ledger_bound` report in `unknowns`. Nothing records when an entry was
//! written, so reads come in recall order, not time order.
//! A single claim is looked up by recalling its id, which its own JSON
//! contains.
//!
//! # Trust
//!
//! Nothing here takes a status from a caller. [`Ledger::record`] computes the
//! status from the claim's source and the evidence ports: see the trust rules in `earn.rs`.

mod earn;
mod effective;
mod types;

use std::collections::BTreeMap;
use std::sync::Arc;

use supermath_tool::{CallContext, CallLog};

use crate::error::{Error, Result};
use crate::evidence::LeanEvidence;
use crate::memory::{self, Memory, Project as MemoryProject};
use crate::types::{Alignment, BoundSpec, Direction, Entry, Kind, Source};

pub use effective::Effective;
pub(crate) use types::Recalled;
pub use types::{BoundOutcome, BoundRequest, Claim, Recorded};

/// How many recent notes a read recalls.
pub const DEFAULT_WINDOW: u64 = 200;

/// The claim ledger of every project in one `CortexDB` deployment.
#[derive(Clone)]
pub struct Ledger {
    memory: Memory,
    evidence: Arc<dyn LeanEvidence>,
    calls: Arc<dyn CallLog>,
    window: u64,
}

impl std::fmt::Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ledger")
            .field("memory", &self.memory)
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

impl Ledger {
    /// A ledger over `memory`, promoting `lean` sources with `evidence` and
    /// `tool` sources against `calls`.
    #[must_use]
    pub fn new(memory: Memory, evidence: Arc<dyn LeanEvidence>, calls: Arc<dyn CallLog>) -> Self {
        Self {
            memory,
            evidence,
            calls,
            window: DEFAULT_WINDOW,
        }
    }

    /// Replaces how many recent notes a read recalls.
    #[must_use]
    pub fn with_window(mut self, window: u64) -> Self {
        self.window = window.max(1);
        self
    }

    /// How many recent notes a read recalls.
    #[must_use]
    pub fn window(&self) -> u64 {
        self.window
    }

    /// The id of the claim `kind`, `statement`, and `source` state:
    /// `clm_` and a digest of the three.
    #[must_use]
    pub fn claim_id(kind: Kind, statement: &str, source: &Source) -> String {
        let source = serde_json::to_string(source).unwrap_or_default();
        format!(
            "clm_{}",
            memory::content_digest(&[kind.as_str(), statement, &source])
        )
    }

    /// Every claim in the project's window, one entry per claim (its highest
    /// revision), recalled by similarity to `query`.
    ///
    /// Entries come in the order memory recalled their notes, each claim at
    /// the place of the first of its notes recalled. That is `CortexDB`'s
    /// ranking for `query`, not time: neither an entry nor a recalled note
    /// carries a creation time, and `revision` counts within one claim only.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidProject`] for a slug with no usable characters, and
    /// the memory errors of [`Memory::recall_notes`].
    pub async fn entries(&self, slug: &str, query: &str) -> Result<Vec<Entry>> {
        Ok(self.recall(slug, query).await?.entries)
    }

    /// The entries of [`Ledger::entries`], and whether the recall filled
    /// the window.
    pub(crate) async fn recall(&self, slug: &str, query: &str) -> Result<Recalled> {
        let project = MemoryProject::new(slug)?;
        let notes = self
            .memory
            .recall_notes(&project, query, self.window)
            .await?;
        Ok(Recalled {
            window_full: notes.len() as u64 >= self.window,
            entries: latest_revisions(
                notes
                    .iter()
                    .filter_map(|note| serde_json::from_str::<Entry>(note).ok()),
            ),
        })
    }

    /// The caveat a read that filled the window carries.
    pub(crate) fn window_note(&self) -> String {
        format!(
            "the ledger was read through a window of {} recalled notes; older entries may be missing",
            self.window
        )
    }

    /// The claim with this id, at its highest revision.
    ///
    /// # Errors
    ///
    /// The same as [`Ledger::entries`].
    pub async fn get(&self, slug: &str, id: &str) -> Result<Option<Entry>> {
        let found = self.entries(slug, &format!("claim:{id} {id}")).await?;
        Ok(found.into_iter().find(|entry| entry.id == id))
    }

    /// Records a claim and returns the entry that stands for it.
    ///
    /// The status is computed from the claim's source (see the trust rules in `earn.rs`). Recording
    /// a claim that is already in the ledger at the status it now earns, with
    /// no new `uses`, changes nothing and returns the existing entry; if the
    /// status differs (a re-check, a changed file) a new revision is written
    /// whose `supersedes` names the one it replaces.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArguments`] for an empty statement, an `arxiv`
    /// source without a version and exactly one locator, or a `uses` that
    /// repeats; [`Error::ClaimNotFound`] when a `uses` id is not in the
    /// ledger; [`Error::LinkCycle`] when re-recording a claim with a new
    /// `uses` id that is the claim itself or already reaches it, the rule
    /// [`Ledger::link`] applies; [`Error::MissingFrame`], [`Error::UnrecordedCall`],
    /// [`Error::CallNotUsable`], [`Error::StaleDigest`], and the evidence
    /// errors from [`Ledger`]'s trust rules; and the memory errors.
    pub async fn record(&self, ctx: &CallContext, claim: Claim) -> Result<Recorded> {
        let slug = ctx.project.slug();
        let statement = claim.statement.trim().to_string();
        if statement.is_empty() {
            return Err(Error::InvalidArguments(
                "`statement` must not be empty".into(),
            ));
        }
        earn::check_source(&claim.source)?;
        let mut uses = Vec::new();
        for id in &claim.uses {
            if uses.contains(id) {
                return Err(Error::InvalidArguments(format!("`uses` repeats `{id}`")));
            }
            if self.get(slug, id).await?.is_none() {
                return Err(Error::ClaimNotFound(id.clone()));
            }
            uses.push(id.clone());
        }
        let id = Self::claim_id(claim.kind, &statement, &claim.source);
        let existing = self.get(slug, &id).await?;
        if let Some(old) = &existing {
            for used in uses.iter().filter(|u| !old.uses.contains(u)) {
                self.refuse_cycle(slug, &id, used, BTreeMap::new()).await?;
            }
        }
        let mut earned = self.earn(ctx, &claim).await?;
        if let Some(old) = &existing
            && earned.origin == EvidenceOrigin::Supplied
            && old.status > earned.status
        {
            // Supplied text is not evidence either way: it cannot raise a
            // claim, and it cannot take away a status collected evidence earned.
            earned = earned.keep(old);
        }
        let (revision, supersedes, alignment, merged_uses) = match &existing {
            Some(old)
                if old.status == earned.status && uses.iter().all(|u| old.uses.contains(u)) =>
            {
                return Ok(Recorded {
                    entry: old.clone(),
                    created: false,
                    notes: earned.notes,
                });
            }
            Some(old) => {
                let mut merged = old.uses.clone();
                merged.extend(uses.iter().filter(|u| !old.uses.contains(u)).cloned());
                (
                    old.revision + 1,
                    Some(format!("{}#{}", old.id, old.revision)),
                    old.alignment,
                    merged,
                )
            }
            None => (1, None, Alignment::NotReviewed, uses),
        };
        let entry = Entry {
            id,
            revision,
            kind: claim.kind,
            statement,
            status: earned.status,
            evidence_origin: earned.origin,
            source: claim.source,
            lean: earned.lean,
            alignment,
            uses: merged_uses,
            supersedes,
            rationale: claim.rationale,
            reason: claim.reason,
            refutes: claim.refutes,
            frame: claim.frame,
            bound: claim.bound,
        };
        self.write(slug, &entry).await?;
        Ok(Recorded {
            entry,
            created: true,
            notes: earned.notes,
        })
    }

    /// Records that claim `from` uses claim `to`.
    ///
    /// Linking what is already linked changes nothing.
    ///
    /// # Errors
    ///
    /// [`Error::ClaimNotFound`] when either claim is missing,
    /// [`Error::LinkCycle`] when `to` already reaches `from` (or they are the
    /// same claim), and the memory errors.
    pub async fn link(&self, ctx: &CallContext, from: &str, to: &str) -> Result<Recorded> {
        let slug = ctx.project.slug();
        let Some(source) = self.get(slug, from).await? else {
            return Err(Error::ClaimNotFound(from.to_string()));
        };
        let Some(target) = self.get(slug, to).await? else {
            return Err(Error::ClaimNotFound(to.to_string()));
        };
        let cache = BTreeMap::from([(target.id.clone(), target)]);
        self.refuse_cycle(slug, from, to, cache).await?;
        if source.uses.iter().any(|used| used == to) {
            return Ok(Recorded {
                entry: source,
                created: false,
                notes: Vec::new(),
            });
        }
        let mut entry = source.clone();
        entry.revision += 1;
        entry.supersedes = Some(format!("{}#{}", source.id, source.revision));
        entry.uses.push(to.to_string());
        self.write(slug, &entry).await?;
        Ok(Recorded {
            entry,
            created: true,
            notes: Vec::new(),
        })
    }

    /// Sets a claim's alignment: the verdict of an alignment review.
    ///
    /// Only the reviewer in `supermath-subagents` calls this; no tool exposes
    /// it to a model.
    ///
    /// # Errors
    ///
    /// [`Error::ClaimNotFound`] when the claim is missing, and the memory
    /// errors.
    pub async fn set_alignment(
        &self,
        ctx: &CallContext,
        id: &str,
        alignment: Alignment,
    ) -> Result<Entry> {
        let slug = ctx.project.slug();
        let Some(old) = self.get(slug, id).await? else {
            return Err(Error::ClaimNotFound(id.to_string()));
        };
        if old.alignment == alignment {
            return Ok(old);
        }
        let mut entry = old.clone();
        entry.revision += 1;
        entry.supersedes = Some(format!("{}#{}", old.id, old.revision));
        entry.alignment = alignment;
        self.write(slug, &entry).await?;
        Ok(entry)
    }

    /// Records a bound on a named quantity if it improves on the best so far,
    /// and returns the best bound either way.
    ///
    /// A bound that does not improve is kept as a `fact`, not as the bound.
    /// The best bound is found among the bounds one read of the window
    /// recalls; [`BoundOutcome::window_full`] says when that read may have
    /// missed an older one.
    ///
    /// # Errors
    ///
    /// The errors of [`Ledger::record`], and the memory errors of
    /// [`Ledger::effective`] for the best bound.
    pub async fn bound(&self, ctx: &CallContext, request: BoundRequest) -> Result<BoundOutcome> {
        let slug = ctx.project.slug();
        let quantity = request
            .quantity
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if quantity.is_empty() {
            return Err(Error::InvalidArguments(
                "`quantity` must not be empty".into(),
            ));
        }
        let statement = format!(
            "{quantity} {} {}",
            match request.direction {
                Direction::Upper => "≤",
                Direction::Lower => "≥",
            },
            request.value
        );
        let spec = BoundSpec {
            quantity: quantity.clone(),
            direction: request.direction,
            value: request.value.clone(),
        };
        let recalled = self.recall(slug, &quantity).await?;
        let bounds: Vec<Entry> = recalled
            .entries
            .into_iter()
            .filter(|e| e.kind == Kind::Bound)
            .filter(|e| {
                e.bound
                    .as_ref()
                    .is_some_and(|b| b.quantity == quantity && b.direction == request.direction)
            })
            .collect();
        let best_before = bounds
            .iter()
            .filter_map(|e| e.bound.as_ref().map(|b| (e, b)))
            .reduce(|best, next| {
                if improves(request.direction, &next.1.value, &best.1.value) {
                    next
                } else {
                    best
                }
            })
            .map(|(e, _)| e.clone());
        let bound_id = Self::claim_id(Kind::Bound, &statement, &request.source);
        let improved = best_before.as_ref().is_none_or(|best| {
            best.bound
                .as_ref()
                .is_some_and(|b| improves(request.direction, &request.value, &b.value))
        });
        let already_recorded = bounds.iter().any(|e| e.id == bound_id);
        let kind = if improved || already_recorded {
            Kind::Bound
        } else {
            Kind::Fact
        };
        let recorded = self
            .record(
                ctx,
                Claim {
                    kind,
                    statement,
                    source: request.source,
                    uses: Vec::new(),
                    rationale: None,
                    reason: None,
                    refutes: None,
                    frame: request.frame,
                    bound: Some(spec),
                },
            )
            .await?;
        let best = if improved {
            recorded.entry.clone()
        } else {
            best_before.unwrap_or_else(|| recorded.entry.clone())
        };
        let best_effective = self.effective_of(slug, &best).await?;
        Ok(BoundOutcome {
            improved: improved && !already_recorded,
            best,
            best_effective,
            window_full: recalled.window_full,
            recorded,
        })
    }

    async fn write(&self, slug: &str, entry: &Entry) -> Result<String> {
        let project = MemoryProject::new(slug)?;
        let text = serde_json::to_string(entry).map_err(|error| Error::MemoryTransport {
            operation: "remember",
            detail: format!("entry does not serialise: {error}"),
        })?;
        let labels = entry.labels();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.memory.remember(&project, &text, &labels).await
    }

    /// Refuses a `uses` edge from `from` to `to` that would close a cycle:
    /// `to` is `from`, or `to` already reaches `from`. `cache` holds claims
    /// already loaded.
    async fn refuse_cycle(
        &self,
        slug: &str,
        from: &str,
        to: &str,
        cache: BTreeMap<String, Entry>,
    ) -> Result<()> {
        let reach = self.closure(slug, &[to.to_string()], cache).await?;
        if from == to || reach.contains_key(from) {
            return Err(Error::LinkCycle {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        Ok(())
    }

    /// Loads every claim reachable from `roots` through `uses`, taking
    /// claims from `cache` before asking memory. A claim that cannot be
    /// found is absent from the result.
    async fn closure(
        &self,
        slug: &str,
        roots: &[String],
        mut cache: BTreeMap<String, Entry>,
    ) -> Result<BTreeMap<String, Entry>> {
        let mut seen: BTreeMap<String, Entry> = BTreeMap::new();
        let mut pending: Vec<String> = roots.to_vec();
        while let Some(id) = pending.pop() {
            if seen.contains_key(&id) {
                continue;
            }
            let entry = match cache.remove(&id) {
                Some(entry) => Some(entry),
                None => self.get(slug, &id).await?,
            };
            if let Some(entry) = entry {
                pending.extend(entry.uses.iter().cloned());
                seen.insert(id, entry);
            }
        }
        Ok(seen)
    }
}

/// Whether `candidate` is a strictly better bound than `current` in
/// `direction`: smaller for an upper bound, larger for a lower one.
fn improves(
    direction: Direction,
    candidate: &serde_json::Number,
    current: &serde_json::Number,
) -> bool {
    let order = compare(candidate, current);
    match direction {
        Direction::Upper => order == std::cmp::Ordering::Less,
        Direction::Lower => order == std::cmp::Ordering::Greater,
    }
}

/// Orders two JSON numbers exactly when both are integers, by `f64`
/// otherwise.
fn compare(a: &serde_json::Number, b: &serde_json::Number) -> std::cmp::Ordering {
    fn integer(n: &serde_json::Number) -> Option<i128> {
        n.as_i64()
            .map(i128::from)
            .or_else(|| n.as_u64().map(i128::from))
    }
    match (integer(a), integer(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(std::cmp::Ordering::Equal),
    }
}

/// Keeps the highest revision of each claim, in order of first appearance.
fn latest_revisions(entries: impl Iterator<Item = Entry>) -> Vec<Entry> {
    let mut order: Vec<String> = Vec::new();
    let mut latest: BTreeMap<String, Entry> = BTreeMap::new();
    for entry in entries {
        match latest.get(&entry.id) {
            Some(held) if held.revision >= entry.revision => {}
            Some(_) => {
                latest.insert(entry.id.clone(), entry);
            }
            None => {
                order.push(entry.id.clone());
                latest.insert(entry.id.clone(), entry);
            }
        }
    }
    order
        .into_iter()
        .filter_map(|id| latest.remove(&id))
        .collect()
}

#[cfg(test)]
mod test;
