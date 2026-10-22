//! What a dispatch returns.

use supermath_tool::Envelope;

/// One finished call: the envelope, and the id the call log recorded it
/// under. A `tool` source in `ledger_record` quotes that id.
#[derive(Debug, Clone, PartialEq)]
pub struct Dispatched {
    /// The call's id in the call log, such as `call-3`.
    pub call_id: String,
    /// What the tool answered.
    pub envelope: Envelope,
}
