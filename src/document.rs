use super::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait Leches {
  fn data(&self) -> Value;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
  pub data: Value,
  #[serde(skip_deserializing)]
  pub status: Status,
}

impl Leches for Document {
  fn data(&self) -> Value {
    println!("lechessss");
    self.data
  }
}
