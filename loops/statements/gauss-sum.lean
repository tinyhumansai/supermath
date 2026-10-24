import Mathlib

namespace Supermath

/-- Twice the sum of the natural numbers below `n` equals `n * (n - 1)`
(bench case `gauss-sum`; Mathlib's `Finset.sum_range_id_mul_two`). -/
def Conjecture : Prop := ∀ n : ℕ, (∑ i ∈ Finset.range n, i) * 2 = n * (n - 1)

end Supermath
