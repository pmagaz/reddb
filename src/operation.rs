use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub enum Operation {
    Insert,
    Delete,
    Update,
}

impl Default for Operation {
    fn default() -> Self {
        Operation::Insert
    }
}
