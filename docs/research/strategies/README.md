# Strategies we want to adopt

This folder turns the mathematician studies in
[`../math-superagent/`](../math-superagent/README.md) into a list of working
strategies for supermath's solver. The studies describe how eleven
mathematicians (Tao, Grothendieck, Erdős, Gowers, Wiles, Thurston, Zeilberger,
Ramanujan, Scholze, Arnold and Perelman) and one AI-mathematics site
(proofatlas.ai) actually get from a statement to a result. This folder picks
the strategies we think the harness should follow, says where each one comes
from, and says what adopting it would mean in this codebase.

It is still research. Nothing here is a decision until a spec in
[`../../specs/`](../../specs/README.md) or an ADR in
[`../../adr/`](../../adr/) adopts it and links back. What this folder adds over
the source notes is a choice: the source notes record 36 Tao heuristics and
about forty firing rules from the other ten subjects, and a solver that tried
to follow all of them would follow none.

## Where the harness is today

The baseline these strategies are measured against is
[`../../specs/solving-harness.md`](../../specs/solving-harness.md) as
implemented in `crates/supermath-agent`:

- one OpenHuman agent turn per run, driven by a single system prompt
  (`crates/supermath-agent/src/harness/prompt.rs`);
- a reply that must end with `VERDICT: proved`, `VERDICT: refuted` or
  `VERDICT: not-established`, where anything else reads as not established;
- a CortexDB memory scoped to one problem, recalled before the turn and
  written to after it, with the scope built from configuration and the slug
  and never from model output;
- three tools: `recall_memory`, and two deferred integer tools
  (`factor_integer`, `gcd_lcm`) the model finds through `tool_search`.

The current prompt already asks for some of what follows: restate the problem,
try small cases, look for a counterexample as seriously as for a proof, and
never treat running out of ideas as a refutation. That is a good start, and it
is also the weakest form of control we have. The source notes use a status
word for it, *unenforced*: a prompt asks for the behaviour and nothing checks
it. Most of the strategies below are worth writing down precisely because they
can be moved out of the prompt and into something the harness checks.

## How each strategy is written

Every entry has the same parts, so that entries can be compared and cited:

1. What the strategy is, in a paragraph.
2. Who it comes from, with a pointer into the source notes in the form
   `tao/01`§10 or `mathematicians/09`§B1. A short quote is included where the
   source states it well. Quotes are copied from the source notes, which copied
   them from the cited primary sources.
3. How supermath would adopt it, staged as *prompt* (a change to the solver
   prompt), *verdict* (a change to what a run must report), *tool* (a new tool
   or a new check in the harness), or *role* (a separate agent turn with its
   own context). Prompt-only adoption is noted as such, because it is
   unenforced.
4. The guard: how the strategy fails when it is followed without judgement,
   and the check that stops that.

Entries tagged `[INFERRED]` in the source notes are flagged here too. They are
the source authors' reading of a mathematician's practice, not something the
mathematician said, and a spec that rests only on them should say so.

## The files

| File | Covers |
| --- | --- |
| [`01-intake.md`](01-intake.md) | What a run does before it attacks: classify the goal, name the difficulties, compute, look for the counterexample, decide whether the problem is in reach |
| [`02-attack.md`](02-attack.md) | Moves that make progress on the goal: cheating strategically, debts, the decomposition test, ordered moves, changing the setting |
| [`03-failure.md`](03-failure.md) | What a failed attempt is worth: failure coordinates, repaired statements, revived approaches, the stuck policy |
| [`04-trust.md`](04-trust.md) | What a run must show before anyone believes it: priced claims, control by examples, non-claims, statement fidelity, collected evidence |
| [`05-memory.md`](05-memory.md) | What a run leaves behind: the three tiers, shared scratch, the conjecture store, the technique library, the catalogue |
| [`06-settings.md`](06-settings.md) | The five places the mathematicians disagree, which side supermath takes on each, and what we are deliberately not adopting |

Read [`06-settings.md`](06-settings.md) first if you only read one file. The
strategies in `01` to `05` are mostly things every subject would accept. The
settings are the choices a solver makes whether or not anyone writes them
down, and the source studies found that the earlier runtime made all five of
them by default.

## The shape of a run these strategies add up to

Taken together, the strategies describe a run that looks roughly like this.
The numbers point at the file that covers each step.

```text
intake (01)       classify the goal, list the difficulties by name,
                  compute small cases, try to break the statement,
                  check whether it is a known gap or a decidable class
        |
reconnaissance    a counterexample search and a proof attempt share one
(01, 02)          budget; whichever returns first ends the other
        |
attack (02)       cheat strategically, carry debts past blockers,
                  gate every decomposition, test strategies against the
                  obstruction list before running them
        |
failure (03)      record where each attempt failed, repair refuted
                  statements, pair the failure with earlier dead approaches
        |
verdict (04)      proved / refuted / not established, plus what the
                  result does not cover, outstanding obligations, and
                  which checks were collected by the harness
        |
memory (05)       write the settled result, the obstructions and the
                  reusable machinery to separate places
```

Today the harness has the first and last boxes in prompt form and the verdict
box in code. The order of the files is roughly the order we would build in,
with one exception: the verdict additions in `04` are cheap and change the
reply format, so they should land before the reply format has consumers.

## The four agreements

The cross-cutting study (`mathematicians/12`) found four things all eleven
subjects agree on. We treat these as requirements rather than options, and
each one appears in the files below:

1. Name the difficulties before choosing a method. Nobody in the set starts by
   attempting the full statement. See [`01-intake.md`](01-intake.md).
2. A failed attempt carries information, and the run has to extract it rather
   than discard it. See [`03-failure.md`](03-failure.md).
3. The output worth optimising is the one a later worker can pick up. See
   [`05-memory.md`](05-memory.md).
4. Scrutiny scales with what rests on a claim, not with how surprising the
   claim is. See [`04-trust.md`](04-trust.md).

## Sources

All pointers resolve inside [`../math-superagent/`](../math-superagent/README.md),
copied from [senamakel/math-superagent](https://github.com/senamakel/math-superagent)
at `f0b3505`. Those notes talk about that repository's runtime (roles, ledgers,
`src/orchestrator/`); where an entry here mentions one of those mechanisms, it
is describing the earlier design for comparison, not something supermath has.
The engineering recommendations in
[`../rea-and-jacobian/implications-for-supermath.md`](../rea-and-jacobian/implications-for-supermath.md)
overlap with `04-trust.md` and are cited there.
