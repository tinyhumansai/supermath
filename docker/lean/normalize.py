"""Pure functions that turn leanprover-community/repl answers into the
supermath Lean service's replies, and build the Lean commands the service
sends. Nothing here starts a process, so `test_normalize.py` covers it without
Lean installed.

The reply shapes are specified in docs/specs/toolkit/lean/README.md
("The HTTP API"); the Rust client in crates/supermath-lean speaks exactly them.
"""

import json
import os
import re

NAME_RE = re.compile(r"^[A-Za-z_\u0080-￿][A-Za-z0-9_.'!?\u0080-￿]*$")
SLUG_RE = re.compile(r"^[a-z0-9-]+$")
PATH_RE = re.compile(r"^lean/Supermath/[A-Z][A-Za-z0-9]*(/[A-Z][A-Za-z0-9]*)*\.lean$")
IMPORT_RE = re.compile(r"^\s*(?:public\s+|meta\s+|private\s+|all\s+)*import\s+(\S+)")
AXIOMS_RE = re.compile(r"depends on axioms: \[(.*)\]", re.S)
TRY_THIS_RE = re.compile(r"Try this: (.*)")
OPTION_RE = re.compile(r"^(some \(?(-?\d+)\)?|none)$")


def allowed_roots(value):
    """SUPERMATH_ROOTS -> the absolute directories a request's `root` may
    name, colon-separated. Unset or empty allows none."""
    return [os.path.realpath(p) for p in (value or "").split(":") if os.path.isabs(p)]


def project_dir(req, projects, roots):
    """The request's project directory: `root` when it names one inside an
    allowed root, else <projects>/<slug>. Raises ValueError with the reason."""
    slug = req.get("project", "")
    if not isinstance(slug, str) or not SLUG_RE.match(slug):
        raise ValueError("bad project or path")
    root = req.get("root")
    if root is None:
        return os.path.join(projects, slug)
    if not isinstance(root, str) or not os.path.isabs(root):
        raise ValueError("root must be an absolute directory")
    real = os.path.realpath(root)
    if not any(real == allowed or real.startswith(allowed + os.sep) for allowed in roots):
        raise ValueError("root is not inside an allowed directory")
    return real


def valid_name(name):
    return bool(NAME_RE.match(name)) and len(name) <= 200


def flatten_messages(messages):
    """REPL messages -> {severity, line, column, text}."""
    out = []
    for m in messages or []:
        pos = m.get("pos") or {}
        out.append(
            {
                "severity": m.get("severity", "info"),
                "line": pos.get("line", 0),
                "column": pos.get("column", 0),
                "text": m.get("data", ""),
            }
        )
    return out


def flatten_sorries(sorries):
    """REPL sorries -> {line, column, goal} (the proof state stays internal)."""
    out = []
    for s in sorries or []:
        pos = s.get("pos") or {}
        out.append(
            {"line": pos.get("line", 0), "column": pos.get("column", 0), "goal": s.get("goal", "")}
        )
    return out


def project_imports(source):
    """The `Supermath.*` modules named by the import header, in order."""
    mods = []
    for line in source.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("--"):
            continue
        m = IMPORT_RE.match(line)
        if not m:
            break
        if m.group(1).startswith("Supermath"):
            mods.append(m.group(1))
    return mods


def blank_header(source):
    """The source with its import header replaced by blank lines, so a
    pre-loaded `import Mathlib` environment can run it and every line number
    still matches the file."""
    lines = source.split("\n")
    last = -1
    for i, line in enumerate(lines):
        stripped = line.strip()
        if not stripped or stripped.startswith("--"):
            continue
        if IMPORT_RE.match(line) or stripped == "prelude":
            last = i
        else:
            break
    return "\n".join([""] * (last + 1) + lines[last + 1 :])


def axioms_by_target(messages, targets):
    """Reads the answer to `#print axioms t1` / `#print axioms t2` / ... (one
    per line, in order) into {target: [axioms] | None}. A target Lean could
    not resolve maps to None."""
    result = {t: None for t in targets}
    for m in messages or []:
        line = (m.get("pos") or {}).get("line", 0)
        if not 1 <= line <= len(targets):
            continue
        text = m.get("data", "")
        found = AXIOMS_RE.search(text)
        if found:
            result[targets[line - 1]] = [a.strip() for a in found.group(1).split(",") if a.strip()]
        elif "does not depend on any axioms" in text:
            result[targets[line - 1]] = []
    return result


def uses_command(targets):
    """A Lean command that logs, per target, the project constants it uses."""
    names = ", ".join(json.dumps(t) for t in targets)
    return (
        "open Lean Elab Command in\n"
        "run_cmd do\n"
        "  let env ← getEnv\n"
        f"  for s in [{names}] do\n"
        "    match env.find? s.toName with\n"
        "    | none => pure ()\n"
        "    | some ci =>\n"
        "      let used := ci.type.getUsedConstants ++ (ci.value?.map (·.getUsedConstants)).getD #[]\n"
        "      let keep := used.filter fun c => match env.getModuleIdxFor? c with\n"
        "        | none => true\n"
        "        | some i => (env.header.moduleNames[i.toNat]!).getRoot == `Supermath\n"
        '      logInfo m!"SUPERMATH_USES {s} {" ".intercalate (keep.toList.map toString)}"\n'
    )


def parse_uses(messages):
    out = {}
    for m in messages or []:
        text = m.get("data", "")
        if text.startswith("SUPERMATH_USES "):
            parts = text[len("SUPERMATH_USES ") :].split()
            if parts:
                out[parts[0]] = sorted(set(parts[1:]))
    return out


def decl_command(name):
    return (
        "open Lean Elab Command Meta in\n"
        "run_cmd do\n"
        "  let env ← getEnv\n"
        f"  let n := {json.dumps(name)}.toName\n"
        "  match env.find? n with\n"
        '  | none => logInfo "SUPERMATH_NOT_FOUND"\n'
        "  | some ci =>\n"
        "    let inst ← liftCoreM (Meta.isInstance n)\n"
        '    let kind := if isStructure env n then "structure" else if inst then "instance" else\n'
        '      match ci with\n'
        '      | .thmInfo _ => "theorem" | .defnInfo _ => "definition" | .axiomInfo _ => "axiom"\n'
        '      | .opaqueInfo _ => "opaque" | .quotInfo _ => "quotient" | .inductInfo _ => "inductive"\n'
        '      | .ctorInfo _ => "constructor" | .recInfo _ => "recursor"\n'
        "    let doc ← findDocString? env n\n"
        '    let modName := match env.getModuleIdxFor? n with\n'
        '      | some i => toString env.header.moduleNames[i.toNat]!\n'
        '      | none => ""\n'
        '    logInfo m!"SUPERMATH_DECL{(Json.mkObj [("kind", kind), ("doc", doc.getD ""), ("module", modName)]).compress}"\n'
        f"#check {name}\n"
    )


def parse_decl(messages, name):
    """-> reply dict, or None when the declaration does not exist."""
    meta, signature = None, ""
    for m in messages or []:
        text = m.get("data", "")
        if text == "SUPERMATH_NOT_FOUND":
            return None
        if text.startswith("SUPERMATH_DECL"):
            meta = json.loads(text[len("SUPERMATH_DECL") :])
        elif m.get("severity") == "info" and not text.startswith("SUPERMATH_"):
            signature = text
    if meta is None:
        return None
    return {
        "status": "ok",
        "name": name,
        "kind": meta.get("kind", ""),
        "signature": signature,
        "doc": meta.get("doc", ""),
        "module": meta.get("module", ""),
    }


def search_command(fragment, limit):
    return (
        "open Lean Elab Command Meta in\n"
        "run_cmd do\n"
        "  let env ← getEnv\n"
        f"  let q := {json.dumps(fragment.lower())}\n"
        "  let hits := env.constants.fold (init := (#[] : Array Name)) fun acc n _ =>\n"
        "    if !n.isInternal && ((toString n).toLower.splitOn q).length > 1 then acc.push n else acc\n"
        "  let sorted := hits.qsort fun a b => (toString a).length < (toString b).length\n"
        f"  for n in sorted.toList.take {int(limit)} do\n"
        "    let ci := env.find? n |>.get!\n"
        "    let ty ← liftTermElabM (ppExpr ci.type)\n"
        '    let modName := match env.getModuleIdxFor? n with | some i => toString env.header.moduleNames[i.toNat]! | none => ""\n'
        '    logInfo m!"SUPERMATH_ROW{(Json.mkObj [("name", toString n), ("type", toString ty), ("module", modName)]).compress}"\n'
    )


def parse_search(messages):
    rows = []
    for m in messages or []:
        text = m.get("data", "")
        if text.startswith("SUPERMATH_ROW"):
            rows.append(json.loads(text[len("SUPERMATH_ROW") :]))
    return rows


def counter_probe(predicate, domain):
    ty = "ℕ" if domain == "nat" else "ℤ"
    return f"#eval decide ({predicate} (0 : {ty}))"


def counter_chunk(predicate, domain, lo, hi):
    if domain == "nat":
        return f"#eval (List.range' {lo} {hi - lo + 1}).find? (fun n => decide ({predicate} n))"
    return (
        f"#eval ((List.range {hi - lo + 1}).map (fun k => (({lo} : ℤ) + k))).find? "
        f"(fun n => decide ({predicate} n))"
    )


def parse_option(messages):
    """`some 7` / `some (-3)` / `none` -> (found, value)."""
    for m in messages or []:
        found = OPTION_RE.match(m.get("data", "").strip())
        if found:
            return (found.group(2) is not None, int(found.group(2)) if found.group(2) else None)
    return (False, None)


def errors_of(response):
    return [m for m in flatten_messages((response or {}).get("messages")) if m["severity"] == "error"]


def goals_at(response, line, column):
    """The goals before the innermost tactic (or sorry) covering a position,
    from a REPL answer made with `allTactics`. Empty when none covers it."""

    def covers(pos, end):
        start = (pos.get("line", 0), pos.get("column", 0))
        stop = (end.get("line", 0), end.get("column", 0))
        return start <= (line, column) < stop

    best = None
    for t in (response or {}).get("tactics", []):
        if covers(t.get("pos") or {}, t.get("endPos") or {}):
            span = ((t["endPos"]["line"] - t["pos"]["line"]), t["endPos"]["column"] - t["pos"]["column"])
            if best is None or span < best[0]:
                best = (span, t.get("goals", ""))
    for s in (response or {}).get("sorries", []):
        pos = s.get("pos") or {}
        if (pos.get("line"), pos.get("column")) == (line, column):
            return s.get("goal", "")
    return best[1] if best else ""


def find_sorry(response, line, column):
    """The proof state of the sorry at a position, or None."""
    for s in (response or {}).get("sorries", []):
        pos = s.get("pos") or {}
        if (pos.get("line"), pos.get("column")) == (line, column):
            return s.get("proofState")
    return None


def tactic_result(tactic, response):
    """One REPL tactic answer -> {tactic, result, goals, error, term}."""
    out = {"tactic": tactic, "result": "fails", "goals": [], "error": None, "term": None}
    response = response or {}
    errors = [m for m in flatten_messages(response.get("messages")) if m["severity"] == "error"]
    if "message" in response:
        out["error"] = response["message"]
    elif errors:
        out["error"] = errors[0]["text"]
    elif response.get("sorries"):
        out["error"] = "the tactic left a sorry"
    elif response.get("goals") == []:
        out["result"] = "closes"
    elif "goals" in response:
        out["result"] = "progress"
        out["goals"] = response["goals"]
    else:
        out["error"] = "the REPL gave no goals"
    if out["result"] == "closes":
        for m in response.get("messages") or []:
            found = TRY_THIS_RE.search(m.get("data", ""))
            suggestion = found.group(1).strip() if found else ""
            # Only an `exact` suggestion carries a term; the client puts it
            # back as `exact <term>` in the ready edit.
            if suggestion.startswith("exact "):
                out["term"] = suggestion.removeprefix("exact ").strip()
                break
    return out


def crash_reply(code):
    """A `/check` reply for a REPL that died on its own (not the memory
    limit's SIGKILL): the file fails, with one error naming how Lean ended.
    A Lean stack overflow aborts the process with SIGABRT, usually from deep
    recursion in `decide` or kernel reduction on a large term."""
    if code is not None and code < 0:
        import signal
        try:
            how = signal.Signals(-code).name
        except ValueError:
            how = f"signal {-code}"
    else:
        how = f"exit code {code}"
    text = f"Lean crashed while checking this file ({how})."
    if how == "SIGABRT":
        text += (" This is most often a stack overflow: deep recursion in `decide`,"
                 " `rfl`, or kernel reduction on a large term. Split the computation"
                 " or prove it another way.")
    return {
        "status": "ok",
        "messages": [{"severity": "error", "line": 1, "column": 0, "text": text}],
        "sorries": [],
        "axioms": {},
        "uses": {},
    }
