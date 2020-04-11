use core::fmt::Display;
use operation::Operation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

mod record;
mod serializer;
mod store;
use store::Store;
mod operation;
mod storage;

use serializer::Serializer;
use serializer::{JsonSerializer, RonSerializer, YamlSerializer};
use storage::FileStorage;
use storage::Storage;
use store::StoreHM;

pub type JSonDb<T> = RedDb<T, JsonSerializer, FileStorage<JsonSerializer>>;
pub type YamlDb<T> = RedDb<T, YamlSerializer, FileStorage<YamlSerializer>>;
pub type RonDb<T> = RedDb<T, RonSerializer, FileStorage<RonSerializer>>;

#[derive(Debug)]
pub struct RedDb<T, SE, ST> {
  pub store: Store<T, SE>,
  pub storage: ST,
}

impl<'a, T, SE, ST> RedDb<T, SE, ST>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Default + Clone,
  for<'de> SE: Serializer<'de> + Debug,
  for<'de> ST: Storage + Debug,
{
  pub fn new() -> Self {
    let storage = ST::new::<T>().unwrap();
    let data: StoreHM = storage.load_data::<T>();

    Self {
      store: Store::new(data),
      storage: storage,
    }
  }

  pub fn insert_one(&self, value: T) -> Uuid {
    let id = self.store.insert_one(&value);
    self
      .storage
      .save_one((id, value, Operation::Insert))
      .unwrap();
    id
  }

  pub fn find_one(&self, id: &Uuid) -> T {
    self.store.find_one(id)
  }

  pub fn update_one(&'a self, id: &Uuid, new_value: T) -> Uuid {
    let id = self.store.update_one(id, &new_value);
    self
      .storage
      .save_one((id, new_value, Operation::Update))
      .unwrap();
    id
  }

  pub fn delete_one(&self, id: &Uuid) -> Uuid {
    let value = self.store.delete_one(id);
    self
      .storage
      .save_one((*id, value, Operation::Delete))
      .unwrap();
    *id
  }

  pub fn insert(&self, values: Vec<T>) -> usize {
    let values = self.store.insert(values);
    let result = values.len();
    self.storage.save(values).unwrap();
    result
  }

  pub fn find(&self, search: &T) -> Vec<T> {
    self.store.find(search)
  }

  pub fn update(&self, search: &T, new_value: &T) -> usize {
    let values = self.store.update(search, new_value);
    let result = values.len();
    self.storage.save(values).unwrap();
    result
  }

  pub fn delete(&self, search: &T) -> usize {
    let values = self.store.delete(search);
    let result = values.len();
    self.storage.save(values).unwrap();
    result
  }
}
