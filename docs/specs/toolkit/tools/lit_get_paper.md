# `lit_get_paper`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Literature](../literature.md) | primitive | deferred | read | P3 |

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
- If the e-print is PDF-only, the text comes from the PDF, and `unknowns`
  says so: formulas from PDF text are unreliable, and section numbers may be
  guessed.
