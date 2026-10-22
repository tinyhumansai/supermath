# supermath-literature

The `lit_*` tools: find and read papers without a model. The spec is
[`docs/specs/toolkit/literature/`](../../docs/specs/toolkit/literature/README.md).

| Tool | Exposure | Backend |
| --- | --- | --- |
| `lit_search_arxiv` | direct | arXiv Atom API |
| `lit_get_paper` | deferred | arXiv metadata and LaTeX e-print |
| `lit_extract_statements` | deferred | arXiv LaTeX e-print, parsed |
| `lit_citations` | deferred | Semantic Scholar Graph API |
| `lit_oeis` | deferred | OEIS JSON search |

## Design

- `Literature::new(&LiteratureConfig::from_env())` builds the clients;
  `.tools()` returns the five `Arc<dyn Tool>` for a `ToolRegistry`. The
  composition crate shares that one value so the arXiv rate limiter is
  process-wide.
- Every network call goes through one fetcher: disk cache first
  (`SUPERMATH_CACHE/literature`), then, for arXiv only, the `RateLimiter`
  (one request per 3 s), then HTTP. 429 and 503 become `RATE_LIMITED` with
  `retry_after_ms`; nothing is retried silently.
- `Clock` is a port: `SystemClock` in production, `ManualClock` in tests, so
  the limiter and cache ages are tested without waiting.
- The LaTeX reader (`flatten_latex`, `outline`, `extract_statements`) is a
  deterministic parse, not a TeX engine. See the module docs of
  `src/latex/` and `src/statements/` for exactly what is and is not handled.
- PDF text is not extracted. A PDF-only paper is reported in `unknowns`.

## Tests

`cargo test -p supermath-literature` runs unit tests, and integration tests
that point every base URL at a loopback stub serving the recorded responses
in `tests/fixtures/` (an Atom feed, a LaTeX e-print tarball with `\input` and
`\newtheorem` variants, a single gzipped `.tex`, a PDF-only e-print, OEIS and
Semantic Scholar JSON, a 429). `eprint_src/` holds the readable sources of
the tarball; rebuild it with
`(cd tests/fixtures/eprint_src && COPYFILE_DISABLE=1 tar --format ustar -czf ../eprint_2401.00001v2.tar.gz .)`.
`SUPERMATH_LIVE_ARXIV=1 cargo test -p supermath-literature --test live`
searches the real arXiv once.
