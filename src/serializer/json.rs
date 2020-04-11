use super::{Serializer, Serializers};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Json {
  format: Serializers,
}

impl Default for Json {
  fn default() -> Json {
    Json {
      format: Serializers::Json(".json.db".to_owned()),
    }
  }
}

pub type JsonSerializer = Json;

impl<'a> Serializer<'a> for Json {
  fn format(&self) -> &Serializers {
    &self.format
  }

  fn serialize<T>(&self, value: &T) -> Vec<u8>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vector = serde_json::to_vec(value).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
  fn deserialize<T>(&self, value: &Vec<u8>) -> T
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    serde_json::from_slice::<T>(value).unwrap()
  }
}
