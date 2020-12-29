use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub enum Status {
    In,
    Up,
    De,
}

impl Default for Status {
    fn default() -> Self {
        Status::In
    }
}
