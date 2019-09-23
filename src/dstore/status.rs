use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub enum Status {
    Saved,
    NotSaved,
}

impl Default for Status {
    fn default() -> Self {
        Status::Saved
    }
}
