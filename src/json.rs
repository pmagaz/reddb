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

impl<'a> DeSerializer<'a> for Json {
  fn serializer<T>(&self, value: &T) -> Vec<u8>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vector = serde_json::to_vec(value).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
  fn deserializer<T>(&self, value: &Vec<u8>) -> T
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    serde_json::from_slice::<T>(value).unwrap()
  }
  fn from_str<T>(&self, value: String) -> T
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    serde_json::from_str::<T>(&value).unwrap()
  }
}
