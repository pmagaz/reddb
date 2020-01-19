use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub enum Status {
    Saved,
    Deleted,
    Updated,
    NotSaved,
}

impl Default for Status {
    fn default() -> Self {
        Status::Saved
    }
}
