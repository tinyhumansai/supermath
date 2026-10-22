# `calc_factor`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-calc` | [Calc](../README.md) | primitive | deferred | read | P1 |

**Given → returns:** an integer → its factorization.

```json
{ "n": 1000000007 }
```

- Exact prime factorization by trial division for `2 ≤ n ≤ 10^12`
  (`FACTOR_LIMIT`), returned as `[{prime, exponent}]` with a `prime` flag.
- Takes the integer as a JSON number or a decimal string; nothing else is
  parsed.
- For larger integers or anything beyond factorization, a refusal's `next` suggests
  Jacobian's `math.find` when that server is connected
  ([`crates.md`](../../crates.md#consuming-other-mcp-servers)) and `py_run`.
- Returns `{n, factorization, factors: [{prime, exponent}], is_prime,
  divisor_count}`.

## Errors

`INVALID_INPUT` for a value below 2 or not an integer; `BUDGET_EXCEEDED`
above the limit, never a partial factorization.
