use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use uuid::Uuid;

#[derive(Debug, Serialize, Clone, PartialEq, Deserialize)]
pub struct Document<T> {
  pub id: Uuid,
  pub data: T,
}

impl<'a, T> Document<T>
where
  T: Serialize + Deserialize<'a> + Debug,
{
  pub fn new(id: Uuid, data: T) -> Self {
    Self { id, data }
  }
}
