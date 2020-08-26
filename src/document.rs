use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use uuid::Uuid;

#[derive(Debug, Serialize, Clone, PartialEq, Deserialize)]
pub struct Document<T> {
  pub _id: Uuid,
  pub data: T,
}

impl<'a, T> Document<T>
where
  T: Serialize + Deserialize<'a> + Debug,
{
  pub fn new(_id: Uuid, data: T) -> Self {
    Self { _id, data }
  }
}
