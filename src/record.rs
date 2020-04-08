use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

use super::status::Status;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T> {
  pub _id: Uuid,
  pub data: T,
  pub status: Status,
}

impl<'a, T> Record<T>
where
  T: Serialize + Deserialize<'a> + Debug,
{
  pub fn new(id: Uuid, value: T, status: Status) -> Self {
    Self {
      _id: id,
      data: value,
      status: status,
    }
  }
}
