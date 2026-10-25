# Settle {{slug}}

The conjecture is pinned in `lean/Supermath/Statement.lean`, which nobody may
edit:

```lean
{{statement}}
```

The loop is settled when either side's theorem checks in Lean with only the
standard axioms: the prover's `theorem proof : Conjecture` in
`lean/Supermath/Prove.lean`, or the refuter's `theorem refute : ¬ Conjecture`
in `lean/Supermath/Refute.lean`. Short of that, the score rewards a side file
that elaborates with fewer `sorry`s, so a proof skeleton whose lemmas check
one by one is progress.

## Where we are

{{incumbent}}

## What has been tried

{{history}}

## What failed last time

{{failures}}

## Your task

Make one concrete step towards settling the conjecture this epoch, and leave
notes in `notes/` that the next epoch can build on.
