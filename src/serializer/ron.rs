use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};

#[derive(Debug)]
pub struct Ron {
  format: Serializers,
}

impl Default for Ron {
  fn default() -> Ron {
    Ron {
      format: Serializers::Ron(".ron.db".to_owned()),
    }
  }
}

pub type RonSerializer = Ron;

impl<'a> Serializer<'a> for Ron {
  fn format(&self) -> &Serializers {
    &self.format
  }

  fn serialize<T>(&self, value: &T) -> Vec<u8>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vector = ron::ser::to_string(value).unwrap().into_bytes();
    vector.extend("\n".as_bytes());
    vector
  }
  fn deserialize<T>(&self, value: &Vec<u8>) -> T
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    ron::de::from_bytes::<T>(value).unwrap()
  }
}
