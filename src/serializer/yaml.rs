use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};

#[derive(Debug)]
pub struct Yaml {
  format: Serializers,
}

impl Default for Yaml {
  fn default() -> Yaml {
    Yaml {
      format: Serializers::Yaml(".yaml.db".to_owned()),
    }
  }
}

pub type YamlSerializer = Yaml;

impl<'a> Serializer<'a> for Yaml {
  fn format(&self) -> &Serializers {
    &self.format
  }

  fn serialize<T>(&self, value: &T) -> Vec<u8>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vector = serde_yaml::to_vec(value).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
  fn deserialize<T>(&self, value: &Vec<u8>) -> T
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    serde_yaml::from_slice::<T>(value).unwrap()
  }
}
