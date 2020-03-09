use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use uuid::Uuid;

use super::status::Status;

pub trait Document<T>: Clone + Sized + Debug {
  fn new(value: T) -> Self;
  fn get_id(&self) -> &Uuid;
  fn get_data(&self) -> &T;
  fn set_data(&mut self, data: T) -> &Self;
  fn set_status(&mut self, status: Status) -> &Self;
  fn get_status(&self) -> &Status;
  fn as_u8(&self) -> Vec<u8>;
  fn data_as_value(&self) -> Value;
  fn find_content(&self, value: &T) -> bool;
  fn update_content(&self, value: &T, new_value: &T) -> Value;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Doc<T> {
  pub _id: Uuid,
  pub data: T,
  pub status: Status,
}

impl<'a, T> Document<T> for Doc<T>
where
  T: Clone + Serialize + Deserialize<'a> + Debug,
{
  fn new(value: T) -> Self {
    Self {
      _id: Uuid::new_v4(),
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
  fn set_status(&mut self, status: Status) -> &Self {
    *&mut self.status = status;
    self
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
  fn find_content(&self, query: &T) -> bool {
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
  fn update_content(&self, query: &T, new_value: &T) -> Value {
    //FIXME pass serializer
    let mut doc_object = serde_json::to_value(&self.get_data()).unwrap();
    let query_object = serde_json::to_value(query).unwrap();
    let new_object = serde_json::to_value(new_value).unwrap();
    let query_fields = query_object.as_object().unwrap();
    let mut matches: Vec<i32> = Vec::new();
    for (prop, field) in query_fields.iter() {
      match doc_object.get(prop) {
        Some(val) => {
          if val == field {
            *doc_object.get_mut(prop).unwrap() = new_object.get(prop).unwrap().clone();
            matches.push(1);
          }
        }
        None => (),
      };
    }
    doc_object
    //query_fields.len() == matches.len()
  }
  fn as_u8(&self) -> Vec<u8> {
    //FIXME pass serializer
    let mut vector = serde_json::to_vec(&self).unwrap();
    vector.extend("\n".as_bytes());
    vector
  }
}
