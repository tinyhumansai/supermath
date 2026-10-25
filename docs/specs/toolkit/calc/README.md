# Calc: small exact arithmetic

- **Status:** Implemented
- **Crate:** `supermath-calc`
- **Parent:** [`../README.md`](../README.md)

The two arithmetic tools the first scaffold shipped, renamed and moved to
their own crate. They answer in microseconds with no service behind them,
so the agent has exact arithmetic even when the
[Python service](../python/README.md) is down. Anything larger belongs in
[`py_run`](../python/tools/py_run.md).

## Crate

| | |
| --- | --- |
| Name | `supermath-calc` |
| Depends on | `supermath-tool` |
| Public API | `factorize`, `gcd`, `FACTOR_LIMIT`, and the tools below |
| Configuration | none |
| Tests | answers and refusals for each tool |

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`calc_factor`](tools/calc_factor.md) | an integer → its factorization |
| [`calc_gcd_lcm`](tools/calc_gcd_lcm.md) | two integers → gcd and lcm |
