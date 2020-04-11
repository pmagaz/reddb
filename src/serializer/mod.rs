use serde::{Deserialize, Serialize};
use std::default::Default;

mod json;
mod ron;
mod yaml;

pub use self::ron::RonSerializer;
pub use json::JsonSerializer;
pub use yaml::YamlSerializer;

#[derive(Debug, Clone)]
pub enum Serializers {
  Json(String),
  Yaml(String),
  Ron(String),
}

pub trait Serializer<'a>: Default {
  fn format(&self) -> &Serializers;
  fn serialize<T>(&self, val: &T) -> Vec<u8>
  where
    for<'de> T: Serialize + Deserialize<'de>;

  fn deserialize<T>(&self, val: &Vec<u8>) -> T
  where
    for<'de> T: Serialize + Deserialize<'de>;
}
