use std::io::Read;

pub use serde::de::{Deserialize, DeserializeOwned};
use serde::Serialize;

pub trait DeSerializer<'a, T: Serialize + Deserialize<'a>>:
  ::std::default::Default + Send + Sync + Clone
{
  fn serialize(&self, val: &T) -> Vec<u8>;
  fn deserialize<R: Read>(&self, s: R) -> T;
}

#[derive(Debug, Default, Clone)]
pub struct Json;

impl<'a, T: Serialize + DeserializeOwned> DeSerializer<'a, T> for Json {
  fn serialize(&self, value: &T) -> Vec<u8> {
    println!("Json!");
    serde_json::to_vec(value).unwrap()
  }
  fn deserialize<R: Read>(&self, value: R) -> T {
    serde_json::from_reader(value).unwrap()
  }
}
