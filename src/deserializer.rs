use serde::{Deserialize, Serialize};
use std::default::Default;

pub trait DeSerializer<'a, D>: Default + Clone
where
  D: Serialize + Deserialize<'a>,
{
  fn serializer(&self, val: &D) -> Vec<u8>;
  fn deserializer(&self, val: &str) -> D;
}
