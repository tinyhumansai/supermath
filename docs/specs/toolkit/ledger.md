# Ledger tools

- **Status:** Draft
- **Parent:** [`README.md`](README.md)

The ledger is the project's memory of what it knows and how well it knows
it: facts read from papers, claims stated in Lean, bounds found by search,
leads, and dead ends. It lives in CortexDB, through the existing memory
client, which moves to `crates/supermath-mcp/src/memory/`
([`crates.md`](crates.md)), so `recall_memory` finds it
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
- `web`: `{url, title, class, retrieved_at}`, for a page a `web_*` tool
  surfaced; it caps the claim at `asserted` ([`web.md`](web.md#status-rules));
- `supplied`: free text, for anything that arrived as text.

`lean`, when present, is the declaration that states the claim and the
latest verdict [`lean_check`](tools/lean_check.md) returned for it, with its
provenance.

## Status

The statuses and what each needs are in
[`README.md`](README.md#claim-status). Who may change them:

| Transition | Allowed when |
| --- | --- |
| → `asserted` | always; the default on record |
| → `cited` | `source` is `arxiv` |
| → `checked` | `source` is a `tool` call this `supermath-mcp` process made, with `evidence_origin: collected` |
| → `formalised` | `supermath-mcp` ran `lean_check` on `lean.declaration` in this process and got `checked` with no debt |
| any → lower | a re-check failed or a Mathlib bump broke it; recorded with `supersedes` |

`ledger_record` is called by a model or an MCP client, so it never takes a
status from its arguments: a caller that sends `"status": "formalised"`
gets `FORBIDDEN`. Given a `lean` source, `ledger_record` runs `lean_check`
on the declaration itself and records the status that check earns; given a
`tool` source, it accepts only a call id from this process's own call log.
The rule holds for every caller, our runner included. This is the
`evidence_origin` rule from
[`strategies/04-trust.md`](../../research/strategies/04-trust.md).

### Alignment

Whether a Lean statement says what the informal statement says is not
something a kernel checks. `alignment` is `not_reviewed` on every new Lean
claim, and only [`agent_review_alignment`](tools/agent_review_alignment.md)
changes it, to `accepted` or `disputed`. A `formalised` claim with
`disputed` alignment is a theorem about something else; the solving loop
must treat it as unproved.

## Edges

`uses` lists the claims a claim depends on. With Lean claims it mirrors the
declaration graph [`lean_blueprint`](tools/lean_blueprint.md) computes; with
cited facts it records what a paper's proof relies on, from
[`lit_extract_statements`](tools/lit_extract_statements.md). A claim
is no stronger than the weakest claim it uses: the ledger reports a claim's
**effective status** as the minimum over everything it reaches.

## Scopes

```text
{root}/project:{slug}/app:brain       # this project's ledger (exists)
{root}/library/app:brain              # facts reusable across projects
```

The library scope holds `fact` entries whose source is a paper or Mathlib,
and is written only by `supermath-mcp` when a project's fact is promoted,
never by a caller ([`strategies/05-memory.md`](../../research/strategies/05-memory.md)).
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

Each tool is specified in its own file under [`tools/`](tools/README.md).

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

1. Does CortexDB recall filter by label exactly, or does `ledger_claims`
   need a local index rebuilt from CortexDB?
2. When should a project's fact be promoted to the library: on
   `formalised` and `accepted`, or also on `cited`?
