use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Display};
use uuid::Uuid;

use super::operation::Operation;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T> {
  pub _id: Uuid,
  pub data: T,
  pub status: Operation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Empty;

impl fmt::Display for Empty {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({})", self)
  }
}

impl<'a, T> Record<T>
where
  T: Serialize + Deserialize<'a> + Debug,
{
  pub fn new(id: Uuid, value: T, status: Operation) -> Self {
    Self {
      _id: id,
      data: value,
      status: status,
    }
  }
}
