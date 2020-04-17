use failure::{Error, ResultExt};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};
use crate::error::RdStoreErrorKind;

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

  fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vec = serde_json::to_vec(value)?;
    vec.extend("\n".as_bytes());
    Ok(vec)
  }
  fn deserialize<T>(&self, value: &Vec<u8>) -> Result<T, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let vec = serde_json::from_slice::<T>(value)?;
    Ok(vec)
  }
}
