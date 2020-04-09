use core::fmt::Display;
use operation::Operation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

mod record;
mod store;
use store::Store;
mod deserializer;
mod json;
mod operation;
mod storage;
use deserializer::DeSerializer;
pub use json::JsonSerializer;

/*
 TODO
 - Clean Db file after load
 - Change to references in search
 - Add Ron and Yaml encoders
 - Unwraps and error handing
 - Configuration
 - Test
 - Benches
*/

#[derive(Debug)]
pub struct RedDb<T, DS> {
  pub store: Store<T, DS>,
}

impl<'a, T, DS> RedDb<T, DS>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Default + Clone,
  for<'de> DS: DeSerializer<'de> + Debug + Clone,
{
  pub fn new() -> Self {
    Self {
      store: Store::new(".db"),
    }
  }

  pub fn insert_one(&self, value: T) -> Uuid {
    let id = self.store.insert_one(&value);
    self.store.persist_one(id, value, Operation::default());
    id
  }

  pub fn find_one(&self, id: &Uuid) -> T {
    self.store.find_one(id)
  }

  pub fn update_one(&'a self, id: &Uuid, new_value: T) -> Uuid {
    let id = self.store.update_one(id, &new_value);
    self.store.persist_one(id, new_value, Operation::Update);
    id
  }

  pub fn delete_one(&self, id: &Uuid) -> Uuid {
    let value = self.store.delete_one(id);
    self.store.persist_one(*id, value, Operation::Delete);
    *id
  }

  pub fn insert(&self, values: Vec<T>) -> usize {
    let docs = self.store.insert(values);
    let result = docs.len();
    self.store.persist(docs);
    result
  }

  pub fn find(&self, search: &T) -> Vec<T> {
    self.store.find(search)
  }

  pub fn update(&self, search: &T, new_value: &T) -> usize {
    let docs = self.store.update(search, new_value);
    let result = docs.len();
    self.store.persist(docs);
    result
  }

  pub fn delete(&self, search: &T) -> usize {
    let docs = self.store.delete(search);
    let result = docs.len();
    self.store.persist(docs);
    result
  }
}
