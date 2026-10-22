//! Outline extraction for Lean and Python.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::outline;
use crate::path::Language;

const LEAN: &str = "import Mathlib

namespace Sendov

/-- A doc comment mentioning sorry. -/
theorem main (a b : ℕ) : a + b = b + a := by
  sorry

private lemma helper : 1 = 1 := rfl

@[simp] def double (n : ℕ) : ℕ :=
  2 * n

instance : Inhabited Nat := ⟨0⟩
example : True := by
  -- sorry in a comment
  trivial

structure Point where
  x : Nat

end Sendov
";

#[test]
fn lean_outline_lists_declarations_with_ranges_and_sorry() {
    let items = outline(Language::Lean, LEAN);
    let summary: Vec<(&str, &str, usize, usize, Option<bool>)> = items
        .iter()
        .map(|i| {
            (
                i.kind.as_str(),
                i.name.as_str(),
                i.start_line,
                i.end_line,
                i.has_sorry,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("theorem", "main", 6, 7, Some(true)),
            ("lemma", "helper", 9, 9, Some(false)),
            ("def", "double", 11, 12, Some(false)),
            ("instance", "<anonymous>", 14, 14, Some(false)),
            ("example", "<anonymous>", 15, 17, Some(false)),
            ("structure", "Point", 19, 20, Some(false)),
        ]
    );
}

#[test]
fn lean_outline_serialises_without_python_fields() {
    let items = outline(Language::Lean, "theorem t : 1 = 1 := by\n  sorry\n");
    assert_eq!(
        serde_json::to_value(&items[0]).unwrap(),
        json!({"kind": "theorem", "name": "t", "start_line": 1, "end_line": 2, "has_sorry": true})
    );
}

#[test]
fn lean_outline_of_a_file_without_declarations_is_empty() {
    assert_eq!(outline(Language::Lean, "import Mathlib\n"), []);
    assert_eq!(outline(Language::Lean, ""), []);
}

const PYTHON: &str = "import math

def search(n: int,
           limit: int = 10) -> int:
    total = 0
    for i in range(n):
        total += i

    return total


class Grid:
    def cell(self):
        return 1

async def fetch():  # comment:
    pass
# trailing
x = 1
";

#[test]
fn python_outline_lists_top_level_functions_and_classes() {
    let items = outline(Language::Python, PYTHON);
    let summary: Vec<(&str, &str, usize, usize)> = items
        .iter()
        .map(|i| (i.kind.as_str(), i.name.as_str(), i.start_line, i.end_line))
        .collect();
    assert_eq!(
        summary,
        [
            ("function", "search", 3, 9),
            ("class", "Grid", 12, 14),
            ("async_function", "fetch", 16, 17),
        ]
    );
    assert_eq!(
        items[0].signature.as_deref(),
        Some("def search(n: int, limit: int = 10) -> int:")
    );
    assert_eq!(items[0].has_sorry, None);
}

#[test]
fn python_signature_without_a_colon_runs_to_the_end_of_the_file() {
    let items = outline(Language::Python, "def broken(\n");
    assert_eq!(items[0].end_line, 1);
}
