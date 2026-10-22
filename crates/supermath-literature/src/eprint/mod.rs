//! Unpacks an arXiv e-print into LaTeX files, or reports that it has none.
//!
//! `https://arxiv.org/e-print/<id>` answers with one of: a gzipped tarball of
//! the source, a single gzipped `.tex` file, a bare tarball, or the PDF itself
//! when the author submitted only a PDF. [`unpack`] tells them apart by their
//! magic bytes, not by header or name.

use std::io::Read;

use flate2::read::GzDecoder;

use crate::error::{Error, Result};
use crate::latex::SourceTree;

/// The largest decompressed e-print, or single member, that is read.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// What an e-print turned out to hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// LaTeX files, ready for [`flatten_latex`](crate::flatten_latex).
    Latex(SourceTree),
    /// No LaTeX: the e-print is a PDF, or an archive without `.tex` files.
    PdfOnly,
}

fn is_gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

fn is_tar(bytes: &[u8]) -> bool {
    bytes.get(257..262) == Some(b"ustar".as_slice())
}

fn is_pdf(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF")
}

fn decode(reason: &impl ToString) -> Error {
    Error::Decode {
        what: "the arxiv e-print".into(),
        reason: reason.to_string(),
    }
}

/// Unpacks the bytes of an e-print.
///
/// # Errors
///
/// [`Error::Decode`] when the bytes are a corrupt archive, or are neither an
/// archive, a PDF, nor text.
pub fn unpack(bytes: &[u8]) -> Result<Source> {
    if is_pdf(bytes) {
        return Ok(Source::PdfOnly);
    }
    if is_gzip(bytes) {
        let mut inner = Vec::new();
        GzDecoder::new(bytes)
            .take(MAX_BYTES)
            .read_to_end(&mut inner)
            .map_err(|e| decode(&e))?;
        return unpack_plain(&inner);
    }
    unpack_plain(bytes)
}

fn unpack_plain(bytes: &[u8]) -> Result<Source> {
    if is_pdf(bytes) {
        Ok(Source::PdfOnly)
    } else if is_tar(bytes) {
        untar(bytes)
    } else if looks_like_text(bytes) {
        let mut tree = SourceTree::new();
        tree.insert("main.tex", String::from_utf8_lossy(bytes).into_owned());
        Ok(Source::Latex(tree))
    } else {
        Err(decode(&"not an archive, a pdf, or text"))
    }
}

/// Latin-1 sources are mostly printable ASCII with a few high bytes. Sources
/// edited on DOS may also carry its end-of-file marker, Ctrl-Z (`0x1a`).
fn looks_like_text(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| !matches!(*b, 0..=8 | 14..=25 | 27..=31))
}

fn untar(bytes: &[u8]) -> Result<Source> {
    let mut archive = tar::Archive::new(bytes);
    let mut tree = SourceTree::new();
    for entry in archive.entries().map_err(|e| decode(&e))? {
        let mut entry = entry.map_err(|e| decode(&e))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| decode(&e))?
            .to_string_lossy()
            .into_owned();
        let is_tex = std::path::Path::new(&path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tex") || ext.eq_ignore_ascii_case("ltx"));
        if !is_tex {
            continue;
        }
        let mut raw = Vec::new();
        (&mut entry)
            .take(MAX_BYTES)
            .read_to_end(&mut raw)
            .map_err(|e| decode(&e))?;
        tree.insert(&path, String::from_utf8_lossy(&raw).into_owned());
    }
    if tree.is_empty() {
        Ok(Source::PdfOnly)
    } else {
        Ok(Source::Latex(tree))
    }
}

#[cfg(test)]
mod test;
