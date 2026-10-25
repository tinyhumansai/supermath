import Mathlib

/-!
# Erdős Problem 779

A conjecture of Marian Deaconescu (Amer. Math. Monthly, see p. 120 of
https://doi.org/10.2307/2975810), as stated in google-deepmind/formal-conjectures,
`FormalConjectures/ErdosProblems/779.lean`. `Nat.nth Nat.Prime i` is the
`(i+1)`-th prime, so `primorial n` is the product of the first `n + 1` primes
and `Nat.nth Nat.Prime n` is the largest prime dividing it. The case `n = 0`
(`P = 2`) is false and excluded. Open; verified numerically for `n ≤ 1000`.

Besides `Conjecture`, this file pins a ladder of structural statements. A
loop scores a rung only when a theorem of exactly that type checks.
-/

namespace Supermath

/-- The product of the first `n + 1` primes, `p₀ p₁ ⋯ pₙ`. -/
noncomputable def primorial (n : ℕ) : ℕ :=
  ∏ i ∈ Finset.range (n + 1), Nat.nth Nat.Prime i

/-- Erdős 779 at `n`: a prime `p` with `pₙ < p < P` and `P + p` prime. -/
def Holds (n : ℕ) : Prop :=
  ∃ p, p.Prime ∧ (primorial n + p).Prime ∧ Nat.nth Nat.Prime n < p ∧ p < primorial n

/-- Erdős Problem 779: it holds for every `n ≥ 1`. -/
def Conjecture : Prop :=
  ∀ n, 1 ≤ n → Holds n

/-- Rung 1, a reformulation: the conjecture at `n` is a prime `q` in
`(P + pₙ, 2P)` whose distance `q - P` from the primorial is itself prime. -/
def Rung1 : Prop :=
  ∀ n, 1 ≤ n → (Holds n ↔ ∃ q, q.Prime ∧ primorial n + Nat.nth Nat.Prime n < q ∧
    q < 2 * primorial n ∧ (q - primorial n).Prime)

/-- Rung 2, a conditional reduction: a prime in the short interval
`(P + 1, P + pₙ₊₁²)` after every primorial (a Cramér-strength hypothesis)
implies the conjecture. -/
def Rung2 : Prop :=
  (∀ n, 1 ≤ n → ∃ q, q.Prime ∧ primorial n + 1 < q ∧
    q < primorial n + Nat.nth Nat.Prime (n + 1) ^ 2) → Conjecture

/-- Rung 3, unconditional and open: the conjecture holds for infinitely many
`n`. -/
def Rung3 : Prop :=
  Set.Infinite {n | Holds n}

/-- Rung 4, unconditional and open, a Chen-type weakening: for every `n ≥ 1`
some prime `p` with `pₙ < p < P` makes `P + p` a prime or a product of two
primes. -/
def Rung4 : Prop :=
  ∀ n, 1 ≤ n → ∃ p, p.Prime ∧ Nat.nth Nat.Prime n < p ∧ p < primorial n ∧
    (primorial n + p).primeFactorsList.length ≤ 2

end Supermath
