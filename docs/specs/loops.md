# Conjecture loops on rsi-ralph

**Status:** Accepted. **Owner:** supermath maintainers.

Related:
- [`docs/research/rsi-ralph.md`](../research/rsi-ralph.md): what the design relies on.
- [`toolkit/mcp/`](toolkit/mcp/README.md): `supermath-mcp`.
- [ADR 0010](../adr/0010-run-on-rsi-ralph.md): why the agents run in
  rsi-ralph.

## Problem

supermath's runner (`supermath-agent`) and its agentic tools
(`supermath-subagents`), both now removed, embedded OpenHuman. They carried
its submodule, its `[patch]` table, a Node/TinyBus Docker stage, and a
Ralph-style loop of their own: send a turn, send a continue prompt, stop at
`STATUS: DONE`.

That loop never measured progress. A turn can end "done" with nothing
checked, and nothing compares one attempt with the next.

[`rsi-ralph`](https://github.com/tinyhumansai/rsi-ralph) already runs hives of
OpenHuman agents against a subject, scores every candidate with commands the
loop folder names, and promotes only a measured improvement.

## Goals and non-goals

- **Goal: one conjecture, one loop folder.** The `rsi-ralph` binary runs it;
  supermath supplies the tools, the scorer and the template, and links no
  OpenHuman of its own.
- **Goal: the toolkit as executable tools.** Each tool call is one
  `supermath-mcp call` process, with the same envelopes, trust rules and
  ledger as over MCP.
- **Goal: Lean decides acceptance.** A seat's reply is never a verdict.
- **Non-goal: improving supermath's own prompts or hive across problems** (a
  meta-loop over the formalize bench). It can be built on the same pieces
  later.
- **Goal: no OpenHuman in this repository.** No crate may depend on an
  `openhuman*` crate (`scripts/check-crate-deps.sh`).

## Behavior

### `supermath-mcp loop new <slug> <statement.lean> [dir]`

Writes `dir` (default `loops/<slug>`) from the template compiled into the
binary (`crates/supermath-mcp/template/`). It refuses an existing folder, and
a statement that does not declare `def Conjecture` inside `namespace Supermath`.

```text
rsi.json        dir subject; editable lean/Supermath/, python/work/, notes/, .log/
                (the call and write logs every tool call appends to);
                frozen Statement.lean, lakefile.toml, lean-toolchain;
                scorer `supermath-mcp score proof <slug> [--env-file F]`,
                decided by seeds [1, 2], maximize, target 1.0;
                models on Surplus (keys.openrouter = SURPLUS_API_KEY, provider "*")
brief.md        the statement, and rsi-ralph's {{incumbent}} {{history}} {{failures}}
hive/           seats lead (incumbent, read-only), prover and refuter (each
                its own candidate); `tools` = every toolkit tool but file_*
                and web_search, as
                `supermath-mcp call <tool> <slug> --project-dir . [--env-file F]`;
                toolpacks lean, python, literature, ledger, ledger-read
subject/lean/Supermath/{Statement,Prove,Refute}.lean, lakefile.toml, lean-toolchain
subject/notes/
```

- **The program.** Every command in the folder runs the binary that wrote it,
  by absolute path, with the same absolute `--env-file`. A generated folder is
  therefore local to one machine, and only the statements are committed
  (`loops/statements/`).
- **Files and search.** Seats edit files with their harness's own tools in
  their checkout. The toolkit's `file_*` tools and `web_search` are left out,
  because OpenHuman registers `file_read`, `file_write` and `web_search`
  itself and refuses a second tool of the same name. `web_research` and
  `web_read` stay.

### `supermath-mcp score proof <project>`

The rsi-ralph scorer. It reads `RSI_CANDIDATE` and `RSI_OUT` and refuses to
run without them.

1. **Copy.** It copies the candidate's `lean/` and `python/` to
   `$RSI_OUT/project`, leaving out `.lake` and symlinks. It never writes the
   candidate.
2. **Verdict files.** For each side whose file exists, it writes a verdict
   file:
   - `ScoreProved.lean`: `theorem settled_proved : Conjecture := proof`;
   - `ScoreRefuted.lean`: `theorem settled_refuted : ¬ Conjecture := refute`.
3. **Checks.** It runs `lean_check` on the side's file, then on the verdict
   file, against the copy, with a 600 s budget.
4. **Score.**
   - `1.0` when a verdict file is `checked` with no debt;
   - `0.75` when it is `checked` with debt;
   - otherwise the best side's `0.2 + 0.3 / (1 + sorries)` if its file
     elaborates, or `0.1 / (1 + errors)` if it does not.
5. **Output.** It writes `result.json` with `score`; `failures` (Lean errors,
   open goals, and a `wrong_statement` entry when a clean file's theorem lacks
   the pinned type); and `metrics` (`prove_sorries`, `prove_errors`,
   `refute_sorries`, `refute_errors`, `settled`). It prints
   `METRIC proof=<score>`.

### Running a loop

A loop needs an `rsi-ralph` built from `e40c7e3` (tinyhumansai/rsi-ralph#75,
which attaches a hive's executable tools to its seats) or later; there is no
tagged release yet.

```sh
docker compose -f compose.yaml -f compose.rsi.yaml --profile host up -d lean python cortexdb
supermath-mcp loop new gauss-sum loops/statements/gauss-sum.lean --env-file .env
export TINYRSI_UPSTREAM="$SURPLUS_URL"     # e.g. https://api.surplusintelligence.ai/v1
rsi-ralph check loops/gauss-sum
rsi-ralph run loops/gauss-sum
```

`compose.rsi.yaml` mounts `$TINYRSI_HOME` into the Lean and Python services
and sets `SUPERMATH_ROOTS`, so a `--project-dir` inside the run store can be
checked ([`toolkit/lean/`](toolkit/lean/README.md#the-http-api)).

## Invariants

- **Fixed statement.** The conjecture is fixed by the frozen
  `Statement.lean`. A candidate that edits it, the lakefile or the toolchain
  fails rsi-ralph's surface check.
- **The scorer only trusts Lean.** It accepts a side only through a file it
  wrote itself, whose theorem has exactly the pinned type, checked by the same
  service and axiom allowlist as `lean_check`.
- **A project comes from the command line.** The scorer, the declarations and
  `loop new` all name it; no tool argument can.
- **`call` is an answer, not a status.** It exits 0 for every envelope, and
  non-zero only for a host failure, with nothing on stdout.
- **Calls persist across processes.** The call log of a `--project-dir`
  project is its `.log/calls.jsonl`, so `ledger_record` can cite a call made
  by an earlier process of the same checkout.

## Acceptance criteria

- `rsi-ralph check` accepts a folder `loop new` writes.
- `score proof` on a candidate whose `Prove.lean` proves the conjecture prints
  `METRIC proof=1`, and on the template's `sorry` stubs scores `0.35`.
- In a run, seats call `lean_check` and the other tools as tools, and the
  candidate they edit is the one `lean_check` reads.

## Open questions

- Whether a meta-loop should tune the template's seats and brief on the
  formalize bench (`bench/formalize/`).
- Whether the MCP server (`supermath-mcp serve`), declared in the package's
  `mcp` section, should replace per-call processes once rsi-ralph attaches
  MCP servers to seats.
