use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValue<T> {
  pub key: Uuid,
  pub value: T,
}

impl<'a, T> KeyValue<T>
where
  T: Serialize + Deserialize<'a> + Debug,
{
  pub fn new(key: Uuid, value: T) -> Self {
    Self {
      key: key,
      value: value,
    }
  }
}
