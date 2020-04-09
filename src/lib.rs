use core::fmt::Display;
use operation::Operation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

mod record;
mod store;
use record::Empty;
use store::Store;
mod deserializer;
mod json;
mod operation;
mod storage;
use deserializer::DeSerializer;
pub use json::JsonSerializer;

/*
 TODO
 - Change to references in search
 - Add insert all
 - Add Ron and Yaml encoders
 - Unwraps and error handing
 - Rebuild Db
 - Configuration
 - Test
 - Benches
*/

#[derive(Debug)]
pub struct RedDb<DS> {
  pub store: Store<DS>,
}

impl<'a, DS> RedDb<DS>
where
  for<'de> DS: DeSerializer<'de> + Debug + Clone,
{
  pub fn new() -> Self {
    Self {
      store: Store::new(".db").unwrap(),
    }
  }

  pub fn insert_one<T>(&self, value: T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let id = self.store.insert_one(&value);
    self.store.persist_one(id, value, Operation::default());
    id
  }

  pub fn find_one<T>(&self, id: &Uuid) -> T
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    self.store.find_one(id)
  }

  pub fn update_one<T>(&'a self, id: &Uuid, new_value: T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let id = self.store.update_one(id, &new_value);
    self.store.persist_one(id, new_value, Operation::Update);
    id
  }

  pub fn delete_one(&self, id: &Uuid) -> Uuid {
    let id = self.store.delete_one(id);
    self.store.persist_one(id, Empty, Operation::Delete);
    id
  }

  pub fn insert<T>(&self, values: Vec<T>) -> usize
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let docs = self.store.insert(values);
    let result = docs.len();
    self.store.persist(docs);
    result
  }

  pub fn find<T>(&self, search: &T) -> Vec<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    self.store.find(search)
  }

  pub fn update<T>(&self, search: &T, new_value: &T) -> usize
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let docs = self.store.update(search, new_value);
    let result = docs.len();
    self.store.persist(docs);
    result
  }

  pub fn delete<T>(&self, search: &T) -> usize
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let docs = self.store.delete(search);
    let result = docs.len();
    self.store.persist(docs);
    result
  }
}
