# Ledger tools

- **Status:** Implemented
- **Crate:** `supermath-ledger`
- **Parent:** [`README.md`](../README.md)

The ledger is the project's memory of what it knows and how well it knows
it: facts read from papers, claims stated in Lean, bounds found by search,
leads, and dead ends. It lives in CortexDB, through the existing memory
client, which moves to `crates/supermath-ledger/src/memory/`
([`crates.md`](../crates.md)), so `recall_memory` finds it
by meaning. What the ledger adds is structure: every entry has a kind, a
status, a source, and edges.

## Entries

```json
{
  "id": "clm_7f3a…",
  "revision": 1,
  "kind": "result",
  "statement": "Every polynomial of degree n ≥ 2 with all roots in the closed unit disk …",
  "status": "cited",
  "evidence_origin": "collected",
  "source": { "arxiv": { "id": "2012.04125", "version": "v3", "label": "thm:main" } },
  "lean": null,
  "alignment": "not_reviewed",
  "uses": [],
  "supersedes": null
}
```

| Kind | Holds |
| --- | --- |
| `fact` | a definition, theorem, or technique from the literature |
| `result` | a claim this project established or is establishing |
| `intake` | the problem as stated, its variants, and the assumptions read into it |
| `bound` | the best known bound on a named quantity, with its source |
| `lead` | an idea worth trying, with why |
| `dead_end` | an approach that failed, with the reason, so it is not retried |
| `refutation` | a counterexample, with the statement it `refutes` and the witness |

These follow the note kinds in
[`strategies/05-memory.md`](../../../research/strategies/05-memory.md). One
entry is one finding.

### Sources

`source` is exactly one of:

- `arxiv`: `{id, version, label}` or `{id, version, section}`: the version
  and exactly one locator are required, and an `arxiv` source with neither
  locator, both, or a blank field is `INVALID_INPUT`;
- `lean`: `{path, declaration, file_sha256}`, for a claim stated in a file
  of the project's Lean package ([`lean/`](../lean/README.md#files-not-source));
- `tool`: `{tool, call_id}`, for a computation, with its arguments' digest;
- `web`: `{url, title, class, retrieved_at}`, for a page a `web_*` tool
  surfaced; it caps the claim at `asserted` ([`web/`](../web/README.md#status-rules));
- `supplied`: free text, for anything that arrived as text.

`lean`, when present, is the file and declaration that state the claim and
the latest verdict [`lean_check`](../lean/tools/lean_check.md) returned for them,
with its provenance. A verdict holds only for the digest it was given for:
once the file changes, the claim's effective status falls to `asserted`
until `lean_check` runs on the new digest.

## Status

The statuses and what each needs are in
[`README.md`](../README.md#claim-status). Who may change them:

| Transition | Allowed when |
| --- | --- |
| → `asserted` | always; the default on record |
| → `cited` | `source` is `arxiv` with a version and one locator, and the call's evidence is not supplied |
| → `checked` | `source` is a successful `calc_*`, `py_*`, or `lean_*` call this process logged for this project, with a stated `frame` (without one: `INVALID_INPUT`); any other tool's call stays `asserted` |
| → `formalised` | the `LeanEvidence` port ran `lean_check` on `lean.path` at `lean.file_sha256` in this process and got `checked` with no debt for `lean.declaration` |
| any → lower | a re-check failed or a Mathlib bump broke it; recorded with `supersedes` |

`ledger_record` is called by a model or an MCP client, so it never takes a
status from its arguments: a caller that sends `"status": "formalised"`
(or `cited`, or `checked`) or `"evidence_origin": "collected"` gets
`FORBIDDEN`; `"status": "asserted"` and `"evidence_origin": "supplied"` are
accepted and only pin the claim down, with no check run: the claim is
`asserted` whatever its source, an `arxiv` one included. A call whose
`CallContext` carries `evidence_origin: supplied` likewise earns nothing
above `asserted` (not even `cited`) and runs no check. Given a `lean` source, `ledger_record` runs `lean_check`
on the declaration itself and records the status that check earns; given a
`tool` source, it accepts only a call id from this process's own call log.
A call id that is not in the log is `FORBIDDEN`, as is one that ran in
another project, was a different tool, or did not end `ok`. A `lean` source
whose `file_sha256` is not the digest Lean checked is `CONFLICT`. The rule
holds for every caller, our runner included. This is the
`evidence_origin` rule from
[`strategies/04-trust.md`](../../../research/strategies/04-trust.md).

### Alignment

Whether a Lean statement says what the informal statement says is not
something a kernel checks. `alignment` is `not_reviewed` on every new Lean
claim, and only [`agent_review_alignment`](../subagents/tools/agent_review_alignment.md)
changes it, to `accepted` or `disputed`. A `formalised` claim with
`disputed` alignment is a theorem about something else; the solving loop
must treat it as unproved.

## Edges

`uses` lists the claims a claim depends on. With Lean claims it mirrors the
declaration graph [`lean_blueprint`](../lean/tools/lean_blueprint.md) computes; with
cited facts it records what a paper's proof relies on, from
[`lit_extract_statements`](../literature/tools/lit_extract_statements.md). A claim
is no stronger than the weakest claim it uses: the ledger reports a claim's
**effective status** as the minimum over everything it reaches.

The `uses` graph has no cycles. [`ledger_link`](tools/ledger_link.md)
refuses an edge from a claim to itself or to a claim that already reaches
it, and [`ledger_record`](tools/ledger_record.md) applies the same rule,
with the same `INVALID_INPUT`, when recording an existing claim again adds
`uses` ids it did not have.

A stored `formalised` holds only for the digest it was earned at, so every
read asks `LeanEvidence` again: a changed digest, a check that no longer
earns `formalised`, or a Lean that cannot be reached all give an effective
status of `asserted`, with the reason in `unknowns`. `ledger_claims` reports
the effective status, never the stored one, and `limited_by` lists the chain
of claims from the one asked about to the weakest it reaches.

## Scopes

```text
{root}/project:{slug}/app:brain       # this project's ledger (exists)
{root}/library/app:brain              # facts reusable across projects
```

The library scope holds `fact` entries whose source is a paper or Mathlib,
and is written only by the ledger itself when a project's fact is promoted,
never by a caller ([`strategies/05-memory.md`](../../../research/strategies/05-memory.md)).
Scope strings are built by the host from the project slug, as today.

## Storage

Each entry is written as one CortexDB experience through
`Memory::remember`, with the entry's JSON as its text and labels
`kind:<kind>`, `status:<status>`, and `claim:<id>`. Writes wait for the
`indexed` barrier. An entry is never edited: a change writes a new
revision of the same claim, and readers take the highest `revision`. A
revision keeps the claim's `id` (so `uses` edges stay valid) and its
`supersedes` names the revision it replaces as `<id>#<revision>`.
Every revision is one experience; `status:` and `kind:` labels are for
humans browsing CortexDB, the ledger itself reads the JSON.

`id` is `clm_` followed by the FNV-1a digest the memory client already uses
for idempotency keys, over the kind, statement, and source. Recording the
same finding twice is a no-op that returns the existing id: a repeat that
earns the same status, with no new `uses`, writes nothing. A repeat that
earns a different one (a re-check, a changed verdict) writes the next
revision, which is how `any → lower` is recorded; a repeat with new `uses`
writes the next revision with the union of old and new, after the cycle
rule in [Edges](#edges).

Besides the fields above an entry may carry `rationale` (a `lead`),
`reason` (a `dead_end`), `refutes` (a `refutation`), `frame` (a `tool`
source: the finite range the result covered), and `bound`
(`{quantity, direction, value}`, on `bound` entries).

## Crate

| | |
| --- | --- |
| Name | `supermath-ledger` |
| Depends on | `supermath-tool`, `reqwest`, `serde`, `serde_json`, `tokio`, `async-trait`, `thiserror` |
| Public API | `Memory` (the CortexDB client, moved from `supermath-agent`; its project type is exported as `MemoryProject`), `Config`, `Entry`, `Kind`, `Status`, `Source`, `Ledger`, the evidence ports below, and `tools(&Memory, &Arc<Ledger>)`, which returns the five tools |
| Feature `test-support` | exposes the CortexDB loopback stubs (`memory::stub::{stub, FakeCortex}`) and `FakeLeanEvidence` to other crates' tests |
| Configuration | `CORTEX_API_URL`, `CORTEX_API_KEY`, `CORTEX_SCOPE_ROOT` |
| Tests | the CortexDB loopback stub, and fake evidence ports; every status transition, allowed and refused |

The ledger promotes a claim only on evidence, but it does not depend on the
crates that produce evidence. It defines a port, which the composition in
[`supermath-mcp`](../mcp/README.md) wires to the real domains:

```rust
/// Runs Lean on a file in this process and reports the verdict for one declaration.
#[async_trait]
pub trait LeanEvidence: Send + Sync {
    async fn verify(&self, path: &str, declaration: &str) -> Result<LeanVerdict>;
}

/// `lean_check`'s verdict, reduced to what the ledger needs.
pub struct LeanVerdict {
    pub verdict: LeanVerdictKind,          // checked | checked_with_sorry | failed | unknown
    pub file_sha256: String,               // the digest Lean checked
    pub sorries: u32,
    pub debt: Vec<String>,                 // native_decide / compiler trust
    pub axioms: BTreeMap<String, Vec<String>>, // per target declaration
    pub provenance: serde_json::Value,
}
```

`LeanVerdict::earns_formalised(declaration)` is the one rule: `checked`, no
`sorry`, no debt, and the declaration present with every axiom in the
allowlist (`propext`, `Classical.choice`, `Quot.sound`). A missing file or
declaration is `NOT_FOUND`, Lean down is `BACKEND_UNAVAILABLE`, and a timeout
is `unknown`, none of which records anything.

`tool` sources are checked against `supermath-tool`'s `CallLog`, the record
of every call this process made.

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`ledger_record`](tools/ledger_record.md) | a claim or fact → its id and status |
| [`ledger_claims`](tools/ledger_claims.md) | filters → paged claims |
| [`ledger_link`](tools/ledger_link.md) | two claim ids → a `uses` edge |
| [`ledger_bound`](tools/ledger_bound.md) | a quantity and a bound → the best bound so far |

## Acceptance criteria

- Unit tests cover every status transition, allowed and refused, against
  the memory stub.
- A test shows a model cannot raise a claim to `checked` or `formalised`
  through any tool's arguments.
- Effective status is tested on a chain where a `formalised` claim uses an
  `asserted` one.
- The entry's serde form is pinned in a unit test.

## Open questions

1. ~~Does CortexDB recall filter by label exactly?~~ **Settled: filter
   client-side.** Recall is by similarity and `/v1/recall` takes no label
   predicate we can rely on, so the ledger recalls a window of the project's
   events (`DEFAULT_WINDOW`, 200) ranked by similarity to a query, parses
   each one's text as an entry, skips what is not an entry, groups by `id`,
   keeps the highest `revision`, and filters by kind, status, and alignment
   itself. A single claim is found by recalling its id, which its own JSON
   contains. The limit is the window: when a recall returns as many notes as
   the window holds (counted before non-entries are skipped and revisions
   merged), entries may be missing, and `ledger_claims` and `ledger_bound`
   say so in `unknowns`. Nothing records when an entry was written (an
   entry has no timestamp, the recalled notes carry only their text, and
   `revision` counts within one claim), so reads come in recall order, not
   time order. A local index would remove the limit and give a time order,
   and is the next step if projects grow past it. `status:` and `kind:`
   labels are still written, for people.
2. When should a project's fact be promoted to the library: on
   `formalised` and `accepted`, or also on `cited`?
