"""Tests for normalize.py against answers shaped like leanprover-community/repl's.

    python3 -I -m unittest discover -s docker/lean
"""

import unittest

import normalize as N


def msg(severity, line, column, data):
    return {"severity": severity, "pos": {"line": line, "column": column}, "endPos": {"line": line, "column": column + 1}, "data": data}


class Names(unittest.TestCase):
    def test_names_and_paths(self):
        self.assertTrue(N.valid_name("Nat.Prime.two_le"))
        self.assertTrue(N.valid_name("α₁.β'"))
        for bad in ["", "a b", "a;b", 'a"b', "1a", "x" * 201, "a\nb"]:
            self.assertFalse(N.valid_name(bad), bad)
        self.assertTrue(N.PATH_RE.match("lean/Supermath/Sendov/Main.lean"))
        for bad in ["lean/Supermath/../x.lean", "lean/Supermath/a.lean", "python/work/a.py", "/lean/Supermath/A.lean"]:
            self.assertFalse(N.PATH_RE.match(bad), bad)
        self.assertTrue(N.SLUG_RE.match("my-project-1"))
        self.assertFalse(N.SLUG_RE.match("../x"))


class Header(unittest.TestCase):
    SOURCE = "-- header\nimport Mathlib\nimport Supermath.A\n\ntheorem t : True := trivial\n"

    def test_blank_header_keeps_line_numbers(self):
        blanked = N.blank_header(self.SOURCE)
        self.assertEqual(blanked.count("\n"), self.SOURCE.count("\n"))
        self.assertEqual(blanked.split("\n")[4], "theorem t : True := trivial")
        self.assertNotIn("import", blanked)

    def test_project_imports(self):
        self.assertEqual(N.project_imports(self.SOURCE), ["Supermath.A"])
        self.assertEqual(N.project_imports("import Mathlib\ndef x := 1\nimport Supermath.B\n"), [])

    def test_no_header(self):
        self.assertEqual(N.blank_header("def x := 1\n"), "def x := 1\n")


class Replies(unittest.TestCase):
    def test_messages_and_sorries_are_flattened(self):
        flat = N.flatten_messages([msg("warning", 3, 8, "declaration uses 'sorry'")])
        self.assertEqual(flat, [{"severity": "warning", "line": 3, "column": 8, "text": "declaration uses 'sorry'"}])
        sorries = N.flatten_sorries([{"proofState": 0, "pos": {"line": 4, "column": 2}, "goal": "⊢ True"}])
        self.assertEqual(sorries, [{"line": 4, "column": 2, "goal": "⊢ True"}])
        self.assertEqual(N.flatten_messages(None), [])

    def test_axioms_are_read_by_line(self):
        targets = ["a", "b", "c", "d"]
        messages = [
            msg("info", 1, 0, "'a' depends on axioms: [propext, Classical.choice, Quot.sound]"),
            msg("info", 2, 0, "'b' does not depend on any axioms"),
            msg("error", 3, 14, "unknown constant 'c'"),
        ]
        got = N.axioms_by_target(messages, targets)
        self.assertEqual(got["a"], ["propext", "Classical.choice", "Quot.sound"])
        self.assertEqual(got["b"], [])
        self.assertIsNone(got["c"])
        self.assertIsNone(got["d"])

    def test_uses_round_trip(self):
        out = N.parse_uses([msg("info", 1, 0, "SUPERMATH_USES top mid base base"), msg("info", 2, 0, "other")])
        self.assertEqual(out, {"top": ["base", "mid"]})
        self.assertIn('"top"', N.uses_command(["top"]))

    def test_decl_round_trip(self):
        messages = [
            msg("info", 1, 0, 'SUPERMATH_DECL{"kind":"theorem","doc":"d","module":"Mathlib.X"}'),
            msg("info", 2, 0, "T.f {p : ℕ} : 2 ≤ p"),
        ]
        got = N.parse_decl(messages, "T.f")
        self.assertEqual(got["kind"], "theorem")
        self.assertEqual(got["signature"], "T.f {p : ℕ} : 2 ≤ p")
        self.assertEqual(got["module"], "Mathlib.X")
        self.assertIsNone(N.parse_decl([msg("info", 1, 0, "SUPERMATH_NOT_FOUND")], "x"))
        self.assertIsNone(N.parse_decl([], "x"))
        self.assertIn("#check T.f", N.decl_command("T.f"))

    def test_search_rows(self):
        rows = N.parse_search([msg("info", 1, 0, 'SUPERMATH_ROW{"name":"a","type":"T","module":"M"}')])
        self.assertEqual(rows, [{"name": "a", "type": "T", "module": "M"}])
        self.assertIn('"prime"', N.search_command("Prime", 5))


class Counterexample(unittest.TestCase):
    def test_options(self):
        self.assertEqual(N.parse_option([msg("info", 1, 0, "some 91")]), (True, 91))
        self.assertEqual(N.parse_option([msg("info", 1, 0, "some (-3)")]), (True, -3))
        self.assertEqual(N.parse_option([msg("info", 1, 0, "none")]), (False, None))
        self.assertEqual(N.parse_option([]), (False, None))

    def test_commands(self):
        self.assertIn("List.range' 5 10", N.counter_chunk("P", "nat", 5, 14))
        self.assertIn("((-3 : ℤ) + k)", N.counter_chunk("P", "int", -3, 6))
        self.assertEqual(N.counter_probe("P", "int"), "#eval decide (P (0 : ℤ))")
        self.assertEqual(len(N.errors_of({"messages": [msg("error", 1, 0, "x"), msg("info", 1, 0, "y")]})), 1)


class Goals(unittest.TestCase):
    ANSWER = {
        "tactics": [
            {"pos": {"line": 3, "column": 2}, "endPos": {"line": 6, "column": 0}, "goals": "outer"},
            {"pos": {"line": 4, "column": 2}, "endPos": {"line": 4, "column": 9}, "goals": "inner"},
        ],
        "sorries": [{"proofState": 7, "pos": {"line": 8, "column": 2}, "endPos": {"line": 8, "column": 7}, "goal": "⊢ sorry goal"}],
    }

    def test_innermost_tactic_wins(self):
        self.assertEqual(N.goals_at(self.ANSWER, 4, 4), "inner")
        self.assertEqual(N.goals_at(self.ANSWER, 5, 1), "outer")
        self.assertEqual(N.goals_at(self.ANSWER, 1, 0), "")

    def test_a_sorry_position_gives_its_goal(self):
        self.assertEqual(N.goals_at(self.ANSWER, 8, 2), "⊢ sorry goal")
        self.assertEqual(N.find_sorry(self.ANSWER, 8, 2), 7)
        self.assertIsNone(N.find_sorry(self.ANSWER, 9, 9))


class Tactics(unittest.TestCase):
    def test_closes_progress_fails(self):
        self.assertEqual(N.tactic_result("omega", {"proofState": 1, "goals": []})["result"], "closes")
        progress = N.tactic_result("simp", {"proofState": 2, "goals": ["⊢ q"]})
        self.assertEqual((progress["result"], progress["goals"]), ("progress", ["⊢ q"]))
        failed = N.tactic_result("rfl", {"message": "Lean error:\nrfl failed"})
        self.assertEqual(failed["result"], "fails")
        self.assertIn("rfl failed", failed["error"])
        errored = N.tactic_result("simp", {"goals": [], "messages": [msg("error", 1, 0, "simp made no progress")]})
        self.assertEqual((errored["result"], errored["error"]), ("fails", "simp made no progress"))

    def test_sorry_and_empty_answers_fail(self):
        self.assertEqual(N.tactic_result("t", {"goals": [], "sorries": [{}]})["result"], "fails")
        self.assertEqual(N.tactic_result("t", {})["result"], "fails")
        self.assertEqual(N.tactic_result("t", None)["result"], "fails")

    def test_exact_question_mark_gives_its_term(self):
        answer = {"goals": [], "messages": [msg("info", 1, 0, "Try this: exact Nat.add_comm a b")]}
        self.assertEqual(N.tactic_result("exact?", answer)["term"], "Nat.add_comm a b")

    def test_a_suggestion_that_is_not_an_exact_gives_no_term(self):
        # The client turns a term into `exact <term>`; `simp?`'s suggestion is a tactic.
        answer = {"goals": [], "messages": [msg("info", 1, 0, "Try this: simp only [Nat.add_comm]")]}
        self.assertIsNone(N.tactic_result("simp?", answer)["term"])


if __name__ == "__main__":
    unittest.main()


class Crash(unittest.TestCase):
    def test_a_crashed_check_is_a_failed_file_naming_the_signal(self):
        # Lean aborts on a stack overflow (deep `decide` or kernel reduction).
        # Reported as `timeout`, it told the prover its proof was slow.
        reply = N.crash_reply(-6)
        self.assertEqual(reply["status"], "ok")
        self.assertEqual(len(reply["messages"]), 1)
        error = reply["messages"][0]
        self.assertEqual((error["severity"], error["line"]), ("error", 1))
        self.assertIn("SIGABRT", error["text"])
        self.assertIn("stack overflow", error["text"])
        self.assertEqual((reply["sorries"], reply["axioms"], reply["uses"]), ([], {}, {}))

    def test_another_exit_names_its_code(self):
        self.assertIn("exit code 3", N.crash_reply(3)["messages"][0]["text"])
        self.assertIn("SIGSEGV", N.crash_reply(-11)["messages"][0]["text"])
