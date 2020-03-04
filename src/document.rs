use serde::{Deserialize, Serialize};
use serde_json::{Serializer, Value};
use std::fmt::Debug;
use uuid::Uuid;

use super::status::Status;

pub trait Doc<T>: Clone + Sized + Debug {
  fn new(id: Uuid, value: T) -> Self;
  fn get_id(&self) -> &Uuid;
  fn get_data(&self) -> &T;
  fn set_data(&mut self, data: T) -> &Self;
  //FIXME
  fn set_status(&mut self, status: Status) -> &Status;
  fn get_status(&self) -> &Status;
  fn as_u8(&self) -> Vec<u8>;
  fn data_as_value(&self) -> Value;
  fn match_values(&self, value: &T) -> bool;
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
  fn new(id: Uuid, value: T) -> Self {
    Self {
      _id: id,
      data: value,
      status: Status::default(),
    }
  }
  fn get_id(&self) -> &Uuid {
    &self._id
  }
  fn get_status(&self) -> &Status {
    &self.status
  }
  fn set_status(&mut self, status: Status) -> &Status {
    *&mut self.status = status;
    &self.status
  }
  fn data_as_value(&self) -> Value {
    serde_json::to_value(&self.get_data()).unwrap()
  }
  fn get_data(&self) -> &T {
    &self.data
  }
  fn set_data(&mut self, data: T) -> &Self {
    *&mut self.data = data;
    self
  }
  fn match_values(&self, query: &T) -> bool {
    //FIXME pass serializer
    let doc_object = serde_json::to_value(&self.get_data()).unwrap();
    let query_object = serde_json::to_value(query).unwrap();
    let query_fields = query_object.as_object().unwrap();
    let mut matches: Vec<i32> = Vec::new();
    for (prop, field) in query_fields.iter() {
      match doc_object.get(prop) {
        Some(val) => {
          if val == field {
            matches.push(1);
          }
        }
        None => (),
      };
    }
    query_fields.len() == matches.len()
  }
  fn as_u8(&self) -> Vec<u8> {
    //FIXME pass serializer
    let mut vector = serde_json::to_vec(&self).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
}
