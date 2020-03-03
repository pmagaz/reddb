use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub enum Status {
    Created,
    Deleted,
    Updated,
    NotSaved,
}

impl Default for Status {
    fn default() -> Self {
        Status::Created
    }
}
