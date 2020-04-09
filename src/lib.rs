use core::fmt::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Mutex;
use uuid::Uuid;

mod document;
mod record;
mod store;
use record::{Empty, Record};
use store::Store;
mod deserializer;
mod json;
mod status;
mod storage;
use storage::Storage;

pub use deserializer::DeSerializer;
pub use json::JsonSerializer;
use status::Status;

#[derive(Debug)]
pub struct RedDb<DS> {
  pub store: Store,
  pub serializer: DS,
  pub storage: Storage,
}

impl<'a, DS> RedDb<DS>
where
  for<'de> DS: DeSerializer<'de> + Debug + Clone,
{
  pub fn new() -> Self {
    Self {
      store: Store::new(),
      serializer: DS::default(),
      storage: Storage::new(".db").unwrap(),
    }
  }

  /*
   TODO
   - funcs to store
   - status to operation
  */

  pub fn insert_one<T>(&self, value: T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let mut store = self.store.to_write();
    let id = Uuid::new_v4();
    let data = self.serializer.serializer(&value);
    let _result = store.insert(id, Mutex::new(data));
    self.persist_one(id, value, Status::default());
    id
  }

  pub fn find_one<T>(&self, id: &Uuid) -> T
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let store = self.store.to_read();
    let value = store.get(&id).unwrap();
    let guard = value.lock().unwrap();
    self.serializer.deserializer(&*guard)
  }

  pub fn update_one<T>(&'a self, id: &Uuid, new_value: T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let mut store = self.store.to_write();
    let value = store.get_mut(&id).unwrap();
    let mut guard = value.lock().unwrap();
    *guard = self.serializer.serializer(&*guard);
    self.persist_one(*id, new_value, Status::Updated);
    id.to_owned()
  }

  pub fn delete_one(&self, id: &Uuid) -> Uuid {
    let mut store = self.store.to_write();
    let _result = store.remove(id).unwrap();
    self.persist_one::<Empty>(*id, Empty, Status::Deleted);
    id.to_owned()
  }

  pub fn find_all<T>(&self, query: &T) -> Vec<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let store = self.store.to_read();
    let serialized = self.serializer.serializer(query);
    let docs: Vec<T> = store
      .iter()
      .map(|(_id, value)| value.lock().unwrap())
      .filter(|value| **value == serialized)
      .map(|value| self.serializer.deserializer(&*value))
      .collect();
    docs
  }

  pub fn update_all<T>(&self, query: &T, new_value: T) -> usize
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let mut store = self.store.to_write();
    let serialized = self.serializer.serializer(query);

    let docs: Vec<(Uuid, T, Status)> = store
      .iter_mut()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(_id, mut value)| {
        *value = self.serializer.serializer(&new_value);
        //FIXME
        (*_id, new_value.clone(), Status::Updated)
      })
      .collect();
    let result = docs.len();
    self.persist_many(docs);
    result
  }

  // pub fn delete_all<T>(&self, query: &T, new_value: T) -> usize
  // where
  //   for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  // {
  //   let mut store = self.store.to_write();
  //   let serialized = self.serializer.serializer(query);

  //   let docs: Vec<(Uuid, T, Status)> = store
  //     .iter_mut()
  //     .map(|(_id, value)| (_id, value.lock().unwrap()))
  //     .filter(|(_id, value)| **value == serialized)
  //     .map(|(id, mut value)| {
  //       let _result = store.remove(id).unwrap();
  //       *value = self.serializer.serializer(&new_value);
  //       (*id, new_value.clone(), Status::Updated)
  //     })
  //     .collect();
  //   let result = docs.len();
  //   self.persist_all(docs);
  //   result
  // }

  pub fn persist_many<T>(&self, docs: Vec<(Uuid, T, Status)>)
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display,
  {
    let serialized: Vec<u8> = docs
      .into_iter()
      .map(|(id, value, status)| Record::new(id, value, status))
      .flat_map(|record| self.serializer.serializer(&record))
      .collect();
    &self.storage.write(&serialized);
  }

  pub fn persist_one<T>(&self, id: Uuid, data: T, status: Status)
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display,
  {
    let record = Record::new(id, data, status);
    let serialized = self.serializer.serializer(&record);
    &self.storage.write(&serialized);
  }
}
