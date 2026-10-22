# `calc_factor`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | Compute | primitive | deferred | read | P1 (renamed from `factor_integer`) |

**Given → returns:** an integer → its factorization.

```json
{ "n": 1000000007 }
```

- Exact prime factorization by trial division for `2 ≤ n ≤ 10^12`
  (`FACTOR_LIMIT`), returned as `[{prime, exponent}]` with a `prime` flag.
- Takes the integer as a JSON number or a decimal string; nothing else is
  parsed.
- For larger integers or anything beyond factorization, `next` points at
  Jacobian's `math.find` when that server is connected
  ([`crates.md`](../crates.md#consuming-other-mcp-servers)).

## Errors

`INVALID_INPUT` for a value below 2 or not an integer; `BUDGET_EXCEEDED`
above the limit, never a partial factorization.
