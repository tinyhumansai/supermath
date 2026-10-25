# `calc_factor`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-calc` | [Calc](../README.md) | primitive | deferred | read | P1 |

**Given → returns:** an integer → its factorization.

```json
{ "n": 1000000007 }
```

- Exact prime factorization by trial division for `2 ≤ n ≤ 10^12`
  (`FACTOR_LIMIT`), returned as rows `[{prime, exponent}]` in ascending
  order, with a top-level `is_prime` flag for `n` itself.
- Takes the integer as a JSON number or a decimal string; nothing else is
  parsed. A JSON number past `u64` is still an integer, just too large:
  it is refused like the same digits sent as a string.
- Returns `{n, factorization, factors: [{prime, exponent}], is_prime,
  divisor_count}`.

## Errors

`INVALID_INPUT` for a value below 2 or not an integer (a negative or
fractional number, a non-digit string); `BUDGET_EXCEEDED` above the limit,
whether `n` came as a string or as a JSON number, never a partial
factorization.

The `BUDGET_EXCEEDED` refusal's `next` holds concrete calls, with
arguments, that factor `n` with sympy in the
[Python service](../../python/README.md) instead:

1. `file_write {"path": "python/work/calc_factor.py", "content": "<a
   factor(n) function over sympy.factorint>"}`;
2. `py_run {"path": "python/work/calc_factor.py", "entry": "factor",
   "args": {"n": "<n's digits>"}}`, with `n` as a decimal string so
   nothing is rounded.

A JSON number past `u64` reaches the tool as a float that has already lost
its digits, so that refusal suggests only the `file_write` and its message
asks for `n` as a decimal string. Calc cannot see which other MCP servers
the caller has connected, so it suggests none of them (such as Jacobian's
`math.find`).
