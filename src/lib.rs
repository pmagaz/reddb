use core::fmt::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Mutex;
use uuid::Uuid;

mod document;
mod record;
mod store;
use document::{Doc, Document};
use record::Record;
use store::Store;
mod deserializer;
mod json;
mod status;
mod storage;
use storage::Storage;

pub use deserializer::DeSerializer;
pub use json::JsonSerializer;

type ByteString = Vec<u8>;
type ByteStr = [u8];

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

  pub fn insert<T>(&self, value: T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + Clone + PartialEq,
  {
    let mut store = self.store.to_write();
    let id = Uuid::new_v4();
    let data = self.serializer.serializer(&value);
    let record = Record::new(id, value);
    let bu = self.serializer.serializer(&record);

    &self.storage.write(&bu);
    let _result = store.insert(id, Mutex::new(data));
    id
  }

  pub fn find_one<T>(&self, id: &Uuid) -> T
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + Clone + PartialEq,
  {
    let store = self.store.to_read();
    let value = store.get(&id).unwrap();
    let guard = value.lock().unwrap();
    self.serializer.deserializer(&*guard)
  }

  // pub fn update_one(&'a self, id: &Uuid, new_value: &'a T) -> &T {
  //   let mut store = self.store.to_write();
  //   let value = store.get_mut(&id).unwrap();
  //   let mut guard = value.lock().unwrap();
  //   let leches = new_value.to_owned();
  //   *guard = leches;
  //   new_value
  // }

  // pub fn delete_one(&self, id: &Uuid) -> T {
  //   let mut store = self.store.to_write();
  //   let result = store.remove(id).unwrap();
  //   let value = result.lock().unwrap();
  //   value.to_owned()
  // }

  // pub fn find_all(&self, query: &T) -> Vec<T> {
  //   let store = self.store.to_read();
  //   let docs: Vec<T> = store
  //     .iter()
  //     .map(|(_id, value)| value.lock().unwrap())
  //     .filter(|value| **value == *query)
  //     .map(|value| value.to_owned())
  //     .collect();
  //   docs
  // }

  // pub fn update_all(&self, query: &T, new_value: &T) -> usize {
  //   let mut store = self.store.to_write();

  //   for x in &mut *store {
  //     let (_id, value) = x;
  //     let value = value.lock().unwrap();
  //     println!("ALLL {:?}", value);
  //   }

  //   let docs: Vec<(&Uuid, T)> = store
  //     .iter_mut()
  //     //.map(|(_id, value)| value.lock().unwrap())
  //     //.filter(|value| **value == *query)
  //     //.map(|value| value.to_owned())
  //     .map(|(_id, value)| {
  //       let mut guard = value.lock().unwrap();
  //       if *guard == *query {
  //         *guard = new_value.to_owned();
  //       }
  //       (_id, guard.to_owned())
  //       let leches = self.serializer.serializer(&doc);

  //     })
  //     .collect();
  //   12
  // }
}
