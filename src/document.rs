use super::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
  pub data: Value,
  #[serde(skip_deserializing)]
  pub status: Status,
}
