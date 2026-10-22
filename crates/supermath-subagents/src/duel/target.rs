//! The pinned target: the names, paths, and Lean text both sides attack.
//!
//! Every name here is derived from the claim id by the host, never from model
//! output, so neither side can move the target or the other side's area.

use serde_json::Value;

use super::types::Side;

/// Most characters of the claim id kept in the duel's directory name.
const STEM_CHARS: usize = 16;

/// One claim's duel: where each file lives and what it must declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    /// The ledger claim being settled.
    pub(crate) claim_id: String,
    /// The claim's informal statement.
    pub(crate) statement: String,
    /// The project file that states the claim in Lean.
    pub(crate) lean_path: String,
    /// The declaration there whose type is the conjecture.
    pub(crate) declaration: String,
    /// The digest the claim's Lean verdict holds for.
    pub(crate) lean_sha256: String,
    /// `C` and the claim id's alphanumerics: one Lean path segment.
    stem: String,
}

/// The module a project Lean file defines: `lean/Supermath/A/B.lean` is
/// `Supermath.A.B`.
pub(crate) fn module_of(path: &str) -> Option<String> {
    let inner = path.strip_prefix("lean/")?.strip_suffix(".lean")?;
    (!inner.is_empty()).then(|| inner.replace('/', "."))
}

fn text<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

impl Target {
    /// The target for ledger claim `claim_id`, whose entry is `claim`.
    ///
    /// # Errors
    ///
    /// A message naming the missing step when the claim has no Lean
    /// statement, its alignment is not `accepted`, or its id holds no
    /// letter or digit to name the duel by.
    pub(crate) fn from_claim(claim_id: &str, claim: &Value) -> Result<Self, String> {
        let (Some(lean_path), Some(declaration), Some(lean_sha256)) = (
            text(claim, "/lean/path"),
            text(claim, "/lean/declaration"),
            text(claim, "/lean/file_sha256"),
        ) else {
            return Err(format!(
                "claim {claim_id} has no Lean statement; run agent_formalize, then agent_review_alignment"
            ));
        };
        let alignment = text(claim, "/alignment").unwrap_or("not_reviewed");
        if alignment != "accepted" {
            return Err(format!(
                "claim {claim_id} has alignment {alignment}; agent_settle needs a statement agent_review_alignment accepted"
            ));
        }
        if module_of(lean_path).is_none() {
            return Err(format!(
                "claim {claim_id} is stated in {lean_path}, which is not a project Lean file"
            ));
        }
        let id: String = claim_id
            .strip_prefix("clm_")
            .unwrap_or(claim_id)
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(STEM_CHARS)
            .collect();
        if id.is_empty() {
            return Err(format!(
                "claim id {claim_id} has no letters or digits to name the duel by"
            ));
        }
        Ok(Self {
            claim_id: claim_id.to_owned(),
            statement: text(claim, "/statement").unwrap_or_default().to_owned(),
            lean_path: lean_path.to_owned(),
            declaration: declaration.to_owned(),
            lean_sha256: lean_sha256.to_owned(),
            stem: format!("C{id}"),
        })
    }

    /// The duel's Lean directory, `lean/Supermath/Duel/C<id>`.
    pub(crate) fn dir(&self) -> String {
        format!("lean/Supermath/Duel/{}", self.stem)
    }

    /// The namespace every duel declaration lives in.
    pub(crate) fn namespace(&self) -> String {
        format!("Supermath.Duel.{}", self.stem)
    }

    /// The refuter's Python package directory.
    pub(crate) fn python_dir(&self) -> String {
        format!("python/work/duel_{}", self.stem.to_ascii_lowercase())
    }

    /// The host-written file that pins `Conjecture`.
    pub(crate) fn statement_path(&self) -> String {
        format!("{}/Statement.lean", self.dir())
    }

    /// `Conjecture`, fully qualified.
    pub(crate) fn conjecture(&self) -> String {
        format!("{}.Conjecture", self.namespace())
    }

    /// The text of `Statement.lean`. `type_of%` takes the declaration's type,
    /// never its proof, so a `sorry` there cannot leak into a win.
    pub(crate) fn statement_text(&self) -> String {
        let module = module_of(&self.lean_path).unwrap_or_default();
        format!(
            "import Mathlib\nimport {module}\n\nnamespace {ns}\n\n/-- Ledger claim {id}, pinned by agent_settle. -/\ndef Conjecture : Prop := type_of% @{decl}\n\nend {ns}\n",
            ns = self.namespace(),
            id = self.claim_id,
            decl = self.declaration,
        )
    }

    /// The directory a side's Lean work goes in.
    fn side_dir(&self, side: Side) -> String {
        match side {
            Side::Prover => format!("{}/Prove", self.dir()),
            Side::Refuter => format!("{}/Refute", self.dir()),
        }
    }

    /// The file a side must win in.
    pub(crate) fn main_path(&self, side: Side) -> String {
        format!("{}/Main.lean", self.side_dir(side))
    }

    /// The theorem a side must prove, fully qualified.
    pub(crate) fn theorem(&self, side: Side) -> String {
        match side {
            Side::Prover => format!("{}.proof", self.namespace()),
            Side::Refuter => format!("{}.refute", self.namespace()),
        }
    }

    /// The theorem's declaration line, as the side writes it in the
    /// namespace.
    pub(crate) fn theorem_line(side: Side) -> &'static str {
        match side {
            Side::Prover => "theorem proof : Conjecture",
            Side::Refuter => "theorem refute : ¬ Conjecture",
        }
    }

    /// The host-written file that checks a side's theorem against the
    /// pinned `Conjecture`.
    pub(crate) fn verdict_path(&self, side: Side) -> String {
        match side {
            Side::Prover => format!("{}/Proved.lean", self.dir()),
            Side::Refuter => format!("{}/Refuted.lean", self.dir()),
        }
    }

    /// The verdict file's theorem, fully qualified.
    pub(crate) fn verdict_theorem(&self, side: Side) -> String {
        match side {
            Side::Prover => format!("{}.settled_proved", self.namespace()),
            Side::Refuter => format!("{}.settled_refuted", self.namespace()),
        }
    }

    /// The text of a side's verdict file: it type-checks only if the side's
    /// theorem has exactly the pinned type.
    pub(crate) fn verdict_text(&self, side: Side) -> String {
        let main = module_of(&self.main_path(side)).unwrap_or_default();
        let body = match side {
            Side::Prover => "theorem settled_proved : Conjecture := proof",
            Side::Refuter => "theorem settled_refuted : ¬ Conjecture := refute",
        };
        format!(
            "import {main}\n\nnamespace {ns}\n\n{body}\n\nend {ns}\n",
            ns = self.namespace()
        )
    }

    /// The path prefixes a side may write under.
    pub(crate) fn areas(&self, side: Side) -> Vec<String> {
        let mut areas = vec![format!("{}/", self.side_dir(side))];
        if side == Side::Refuter {
            areas.push(format!("{}/", self.python_dir()));
        }
        areas
    }

    /// The side's goal, in one line.
    pub(crate) fn goal(&self, side: Side) -> String {
        format!(
            "prove `{}` in {}, inside `namespace {}`",
            Self::theorem_line(side),
            self.main_path(side),
            self.namespace()
        )
    }
}
