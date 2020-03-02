use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::From;
use std::fmt::Debug;
use uuid::Uuid;

use super::status::Status;

pub trait Doc<T>: Clone + Sized + Debug {
  fn new(id: Uuid, value: T, status: Status) -> Self;
  fn get_id(&self) -> &Uuid;
  fn get_data(&self) -> &T;
  fn get_status(&self) -> &Status;
  fn as_u8(&self) -> Vec<u8>;
  fn find_values(&self, value: &T) -> &Self;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document<T> {
  pub _id: Uuid,
  pub data: T,
  pub status: Status,
}

impl<'a, T> Doc<T> for Document<T>
where
  T: Clone + Serialize + Deserialize<'a> + Debug,
{
  fn new(id: Uuid, value: T, status: Status) -> Self {
    Self {
      _id: id,
      data: value,
      status: status,
    }
  }
  fn get_id(&self) -> &Uuid {
    &self._id
  }
  fn get_status(&self) -> &Status {
    &self.status
  }
  fn get_data(&self) -> &T {
    &self.data
  }
  fn find_values(&self, value: &T) -> &Self {
    println!("{:?} VALUEEEEE", value);
    //let leches = value as Value;
    //serde_json::from(value);
    // /serde_json::from_
    //serde_json::Serializer::new(writer: W)
    //value.serialize(value);
    //value.serialize(serializer: S)
    //let query_map = value
    &self
  }
  fn as_u8(&self) -> Vec<u8> {
    let mut vector = serde_json::to_vec(&self).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
}
