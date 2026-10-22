# `calc_gcd_lcm`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | Compute | primitive | deferred | read | P1 (renamed from `gcd_lcm`) |

**Given → returns:** two integers → gcd and lcm.

```json
{ "a": 84, "b": 36 }
```

- Exact gcd and lcm of two `u64`. The lcm is computed in `u128` and
  returned as a decimal string, so it never overflows.
- `gcd(0, 0)` is `0` and `lcm` with a zero argument is `0`, stated in the
  description.

## Errors

`INVALID_INPUT` for a negative or non-integer argument.
