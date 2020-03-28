use super::deserializer::DeSerializer;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

pub type Repr = String;

#[derive(Clone, Default, Debug, Serialize, PartialEq, Deserialize)]
struct MyStruct {
  foo: String,
}

#[derive(Debug, Default, Clone)]
pub struct Json;
pub type JsonSerializer = Json;

impl<'a, T> DeSerializer<'a, T> for Json
where
  for<'de> T: Serialize + Deserialize<'de>,
{
  fn serializer(&self, value: &T) -> Vec<u8> {
    let mut vector = serde_json::to_vec(value).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
  fn deserializer(&self, value: &str) -> T {
    serde_json::from_str(value).unwrap()
  }
}
