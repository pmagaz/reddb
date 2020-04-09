use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Display};
use uuid::Uuid;

use super::status::Status;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T> {
  pub _id: Uuid,
  pub data: T,
  pub status: Status,
}

// impl<'de> Deserialize<'de> for Account {
//   fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//   where
//     D: Deserializer<'de>,
//   {
//     let s: &str = Deserialize::deserialize(deserializer)?;
//     // do better hex decoding than this
//     u64::from_str_radix(&s[2..], 16)
//       .map(Account)
//       .map_err(D::Error::custom)
//   }
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct Empty;

impl fmt::Display for Empty {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({})", self)
  }
}

impl<'a, T> Record<T>
where
  T: Serialize + Deserialize<'a> + Debug,
{
  pub fn new(id: Uuid, value: T, status: Status) -> Self {
    Self {
      _id: id,
      data: value,
      status: status,
    }
  }
}
