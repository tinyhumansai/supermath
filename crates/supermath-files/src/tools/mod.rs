//! The four `file_*` tools.
//!
//! Source reaches a project's workspace only through these. They are thin:
//! every rule (paths, imports, digests, atomicity, the log) lives in
//! [`Workspace`](crate::Workspace), and the tools parse arguments, call it,
//! and shape the envelope.

mod file_edit;
mod file_list;
mod file_read;
mod file_write;
mod wire;

use std::sync::Arc;

use supermath_tool::Tool;

use crate::observer::FileObserver;
use crate::workspace::Workspace;

pub use file_edit::FileEdit;
pub use file_list::FileList;
pub use file_read::{FileRead, MAX_READ_LINES};
pub use file_write::FileWrite;

/// All four tools over one shared `workspace`, in the order `file_write`,
/// `file_edit`, `file_read`, `file_list`. `observers` feed `file_list`'s last
/// results; pass the Lean and Python observers, or none.
#[must_use]
pub fn file_tools(
    workspace: &Arc<Workspace>,
    observers: Vec<Arc<dyn FileObserver>>,
) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(FileWrite::new(Arc::clone(workspace))),
        Arc::new(FileEdit::new(Arc::clone(workspace))),
        Arc::new(FileRead::new(Arc::clone(workspace))),
        Arc::new(FileList::new(Arc::clone(workspace), observers)),
    ]
}

#[cfg(test)]
mod test;
