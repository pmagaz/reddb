use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::status::Status;

pub trait Record<T>: Clone {
  fn new(id: Uuid, value: T, status: Status) -> Self;
  fn get_id(&self) -> &Uuid;
  fn get_data(&self) -> &T;
  fn as_u8(&self) -> Vec<u8>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedDbRecord<T> {
  pub _id: Uuid,
  pub data: T,
  pub status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRecord {
  pub _id: Uuid,
  pub data: serde_json::Value,
  pub status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RonRecord {
  pub _id: Uuid,
  pub data: ron::Value,
  pub status: Status,
}

impl<T> Record<T> for RedDbRecord<T>
where
  T: Clone + Serialize,
{
  fn new(id: Uuid, value: T, status: Status) -> Self {
    Self {
      _id: id,
      data: value,
      status: status,
    }
  }
  fn get_id(&self) -> &Uuid {
    &self._id
  }
  fn get_data(&self) -> &T {
    &self.data
  }
  fn as_u8(&self) -> Vec<u8> {
    let mut vector = serde_json::to_vec(&self).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
}
