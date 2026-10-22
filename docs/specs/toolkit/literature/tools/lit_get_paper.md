# `lit_get_paper`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-literature` | [Literature](../README.md) | primitive | deferred | read | P3 |

**Given → returns:** an arXiv id → metadata, section index, a section's text.

Given an arXiv id, returns its metadata and section index, or one section.

```json
{ "arxiv_id": "2310.01234", "section": "3.2" }
```

- Without `section`: metadata, the abstract, and the section tree as
  `{number, title, chars}`.
- With `section`: that section's text, at most 12,000 characters, with a
  `cursor` for the rest.
- The source is the LaTeX e-print, flattened: `\input` followed, macros from
  the preamble expanded where they are simple `\newcommand`s, comments
  removed. Math stays as LaTeX.
- Sections are numbered as LaTeX numbers them (`3`, `3.2`, `A.1` after
  `\appendix`); starred sections have `number: null` and cannot be asked for.
  A section's text includes its subsections. `cursor` is a character offset
  into that section's text, valid only with `section`.
- If the e-print is PDF-only (or arXiv serves no source), no text is read:
  the metadata is returned with `source: "pdf"`, an empty `sections`, and an
  `unknowns` entry saying so; asking for a `section` is `unknown`. PDF text
  extraction is not implemented, because formulas from PDF text are
  unreliable.
- Problems resolving the source (a missing `\input` file, a cycle) are
  listed in `unknowns`.

Errors: `INVALID_INPUT` (a malformed id, or `cursor` without `section`),
`NOT_FOUND` (no such paper or section), `RATE_LIMITED`, `TIMEOUT`,
`BACKEND_UNAVAILABLE`.
