use failure::Error;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::result::Result;

mod json;
mod ron;
mod yaml;

pub use self::ron::RonSerializer;
//use crate::error::Result;
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
  fn serialize<T>(&self, val: &T) -> Result<Vec<u8>, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>;

  fn deserialize<T>(&self, val: &Vec<u8>) -> Result<T, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>;
}