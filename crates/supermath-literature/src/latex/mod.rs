//! Reads LaTeX source: a file tree, flattened to one string with comments
//! removed, `\input` followed, and simple macros expanded.
//!
//! This is a parse, not a TeX engine. It follows `\input` and `\include`,
//! strips `%` comments, and expands macros with no arguments
//! (`\newcommand{\R}{\mathbb{R}}`, `\def`, `\DeclareMathOperator`). Macros with
//! arguments, and `\renewcommand`, are left exactly as written. Math stays as
//! LaTeX.

mod flatten;
mod outline;
pub(crate) mod scan;
mod tree;

pub use flatten::{Flattened, flatten_latex};
pub use outline::{Heading, Outline, outline};
pub use tree::SourceTree;

#[cfg(test)]
mod test;
