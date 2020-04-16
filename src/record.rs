use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

use super::operation::Operation;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T> {
  pub _id: Uuid,
  pub data: T,
  //pub operation: Operation,
}

impl<'a, T> Record<T>
where
  T: Serialize + Deserialize<'a> + Debug,
{
  pub fn new(id: Uuid, value: T) -> Self {
    Self {
      _id: id,
      data: value,
      //operation: operation,
    }
  }
}
