use serde::{Deserialize, Serialize};
use std::default::Default;
use std::io::Read;

pub trait DeSerializer<'a, D>: Default + Clone
where
  D: Serialize + Deserialize<'a>,
{
  fn serializer(&self, val: &D) -> Vec<u8>;
  fn deserializer(&self, val: &str) -> D;
}

#[derive(Debug, Default, Clone)]
pub struct Json;
pub type JsonSerializer = Json;

impl<'a, D> DeSerializer<'a, D> for Json
where
  for<'de> D: Serialize + Deserialize<'de>,
{
  fn serializer(&self, value: &D) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
  }
  fn deserializer(&self, value: &str) -> D {
    serde_json::from_str(value).unwrap()
  }
}
