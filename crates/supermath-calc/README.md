# supermath-calc

Two exact arithmetic tools, `calc_factor` and `calc_gcd_lcm`, implementing
`supermath_tool::Tool`. They answer in microseconds with no service behind
them, so the agent keeps exact arithmetic when the Python service is down.
The spec is [`docs/specs/toolkit/calc/`](../../docs/specs/toolkit/calc/README.md).

## Public surface

| Item | Purpose |
| --- | --- |
| `CalcFactor` | the `calc_factor` tool (unit struct) |
| `CalcGcdLcm` | the `calc_gcd_lcm` tool (unit struct) |
| `factorize`, `gcd`, `FACTOR_LIMIT` | the arithmetic the tools wrap |

## Design constraints

- Inputs are typed integers (a JSON number or a decimal string), never
  expressions, so nothing the model writes is evaluated.
- `calc_factor` refuses above `FACTOR_LIMIT` (10^12) with `BUDGET_EXCEEDED`
  and never returns a partial factorization; trial division is bounded at
  about a million steps.
- `calc_gcd_lcm` computes the lcm in `u128` and returns it as a string.
- No configuration, no environment variables, no I/O.
