use serde::{Deserialize, Serialize};
use std::default::Default;

pub trait DeSerializer<'a>: Default + Clone {
  fn serializer<T>(&self, val: &T) -> Vec<u8>
  where
    for<'de> T: Serialize + Deserialize<'de>;

  fn deserializer<T>(&self, val: &Vec<u8>) -> T
  where
    for<'de> T: Serialize + Deserialize<'de>;
}
