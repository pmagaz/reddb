use serde::{Deserialize, Serialize};

/// Internal WAL operation marker. Never exposed in the public API.
#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub(crate) enum Status {
    In,
    Up,
    De,
}

impl Default for Status {
    fn default() -> Self {
        Status::In
    }
}
