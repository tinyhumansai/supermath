- `lean/Supermath/Statement.lean` pins `Supermath.Conjecture : Prop`. It is
  frozen: a candidate that changes it is discarded.
- The prover proves `theorem proof : Conjecture` in `lean/Supermath/Prove.lean`
  (namespace `Supermath`); the refuter proves `theorem refute : ¬ Conjecture`
  in `lean/Supermath/Refute.lean`. Lemmas go in their own
  `lean/Supermath/<Name>.lean` files and are imported.
- The scorer copies the checkout, checks a host-written file that uses your
  theorem at exactly the pinned type, and reads Lean's verdict. A theorem with
  another name or type does not count. A reply saying something is proved is
  never evidence; only `lean_check` is.
- Score: 1.0 when a side's theorem checks with only `propext`,
  `Classical.choice` and `Quot.sound`; 0.75 with `native_decide` debt;
  otherwise 0.5 / (1 + sorries) for a file that elaborates, and
  0.1 / (1 + errors) for one that does not.
- Check every change with `lean_check`. Work each `sorry` with `lean_goals`
  and `lean_try_tactics`. Find Mathlib names with `lean_search` and
  `lean_decl`. A Python witness is a lead until Lean confirms it.
- Record what you established with `ledger_record`, and read what earlier
  epochs found with `ledger_claims` and `recall_memory`.
