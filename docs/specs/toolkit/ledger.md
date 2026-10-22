# Ledger tools

- **Status:** Draft
- **Parent:** [`README.md`](README.md)

The ledger is the project's memory of what it knows and how well it knows
it: facts read from papers, claims stated in Lean, bounds found by search,
leads, and dead ends. It lives in CortexDB, through the existing memory
client (`crates/supermath-agent/src/memory/`), so `recall_memory` finds it
by meaning. What the ledger adds is structure: every entry has a kind, a
status, a source, and edges.

## Entries

```json
{
  "id": "clm_7f3a…",
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
[`strategies/05-memory.md`](../../research/strategies/05-memory.md). One
entry is one finding.

### Sources

`source` is exactly one of:

- `arxiv`: `{id, version, label}` or `{id, version, section}`;
- `lean`: `{declaration, source_sha256}`, for a claim stated in Lean;
- `tool`: `{tool, call_id}`, for a computation, with its arguments' digest;
- `supplied`: free text, for anything that arrived as text.

`lean`, when present, is the declaration that states the claim and the
latest verdict [`lean_check`](lean.md#lean_check) returned for it, with its
provenance.

## Status

The statuses and what each needs are in
[`README.md`](README.md#claim-status). Who may change them:

| Transition | Allowed when |
| --- | --- |
| → `asserted` | always; the default on record |
| → `cited` | `source` is `arxiv` |
| → `checked` | `source` is a `tool` call this harness made, with `evidence_origin: collected` |
| → `formalised` | the harness ran `lean_check` on `lean.declaration` in this process and got `checked` with no debt |
| any → lower | a re-check failed or a Mathlib bump broke it; recorded with `supersedes` |

`ledger_record` is called by the model, so it can only record `asserted`,
`cited`, `lead`, `dead_end`, or `intake`. `checked` and `formalised` are set
by the harness from the tool result it holds, never from arguments: a model
that sends `"status": "formalised"` gets `FORBIDDEN`. This is the
`evidence_origin` rule from
[`strategies/04-trust.md`](../../research/strategies/04-trust.md).

### Alignment

Whether a Lean statement says what the informal statement says is not
something a kernel checks. `alignment` is `not_reviewed` on every new Lean
claim, and only [`agent_review_alignment`](agents.md#agent_review_alignment)
changes it, to `accepted` or `disputed`. A `formalised` claim with
`disputed` alignment is a theorem about something else; the solving loop
must treat it as unproved.

## Edges

`uses` lists the claims a claim depends on. With Lean claims it mirrors the
declaration graph [`lean_blueprint`](lean.md#lean_blueprint) computes; with
cited facts it records what a paper's proof relies on, from
[`lit_extract_statements`](literature.md#lit_extract_statements). A claim
is no stronger than the weakest claim it uses: the ledger reports a claim's
**effective status** as the minimum over everything it reaches.

## Scopes

```text
{root}/project:{slug}/app:brain       # this project's ledger (exists)
{root}/library/app:brain              # facts reusable across projects
```

The library scope holds `fact` entries whose source is a paper or Mathlib,
and is written only by the harness when a project's fact is promoted, never
by a model ([`strategies/05-memory.md`](../../research/strategies/05-memory.md)).
Scope strings are built by the host from the project slug, as today.

## Storage

Each entry is written as one CortexDB experience through
`Memory::remember`, with the entry's JSON as its text and labels
`kind:<kind>`, `status:<status>`, and `claim:<id>`. Writes wait for the
`indexed` barrier. An entry is never edited: a status change writes a new
entry whose `supersedes` names the old id, and readers take the newest.

`id` is `clm_` followed by the FNV-1a digest the memory client already uses
for idempotency keys, over the kind, statement, and source. Recording the
same finding twice is a no-op that returns the existing id.

## Tools

### `ledger_record`

Given a kind, a statement, and a source, records an entry and returns its
`id` and status. Optional: `uses`, a `lead`'s rationale, a `dead_end`'s
reason. The status is computed, not passed.

### `ledger_claims`

Given filters (`kind`, minimum `status`, `alignment`, or a text query),
returns entries, newest first, as `{id, kind, status, alignment, statement}`
with statements trimmed to 300 characters. `ledger_claims {"id": …}` returns
one entry in full, with its effective status and the chain of claims that
limit it.

### `ledger_link`

Given two claim ids, records that the first uses the second. Refuses a
cycle (`INVALID_INPUT`).

### `ledger_bound`

Given a named quantity, a direction (`upper` or `lower`), a value, and a
source, records it if it improves on the best bound so far, and returns the
best bound either way. Bounds only improve: a worse bound is kept as a
`fact`, not as the bound.

## Acceptance criteria

- Unit tests cover every status transition, allowed and refused, against
  the memory stub.
- A test shows a model cannot raise a claim to `checked` or `formalised`
  through any tool's arguments.
- Effective status is tested on a chain where a `formalised` claim uses an
  `asserted` one.
- The entry's serde form is pinned in a unit test.

## Open questions

1. Does CortexDB recall filter by label exactly, or does `ledger_claims`
   need a local index rebuilt from CortexDB?
2. When should a project's fact be promoted to the library: on
   `formalised` and `accepted`, or also on `cited`?
