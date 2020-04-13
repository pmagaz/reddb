use core::fmt::Display;
use failure::ResultExt;
use operation::Operation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;
mod record;
mod serializer;
mod store;
use store::Store;
mod error;
mod operation;
mod storage;

use error::{RdStoreErrorKind, Result};
use serializer::Serializer;
use serializer::{JsonSerializer, RonSerializer, YamlSerializer};
use storage::FileStorage;
use storage::Storage;
use store::StoreHM;

pub type JSonStore<T> = RedDb<T, JsonSerializer, FileStorage<JsonSerializer>>;
pub type YamlStore<T> = RedDb<T, YamlSerializer, FileStorage<YamlSerializer>>;
pub type RonStore<T> = RedDb<T, RonSerializer, FileStorage<RonSerializer>>;

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
  pub fn new() -> Result<Self> {
    let storage = ST::new::<T>()?;
    let data: StoreHM = storage
      .load_content::<T>()
      .context(RdStoreErrorKind::ContentLoad)?;

    Ok(Self {
      store: Store::new(data),
      storage: storage,
    })
  }

  pub fn insert(&self, value: T) -> Result<Uuid> {
    let id = self.store.insert(&value)?;
    self
      .storage
      .save_one((id, value, Operation::Insert))
      .context(RdStoreErrorKind::DataSave)?;
    Ok(id)
  }

  pub fn find(&self, id: &Uuid) -> Result<T> {
    Ok(self.store.find(id)?)
  }

  pub fn update(&'a self, id: &Uuid, new_value: T) -> Result<Uuid> {
    let id = self.store.update(id, &new_value)?;

    self
      .storage
      .save_one((id, new_value, Operation::Update))
      .context(RdStoreErrorKind::DataSave)?;
    Ok(id)
  }

  pub fn delete(&self, id: &Uuid) -> Result<Uuid> {
    let value = self.store.delete(id)?;
    self
      .storage
      .save_one((*id, value, Operation::Delete))
      .context(RdStoreErrorKind::DataSave)?;
    Ok(*id)
  }

  pub fn insert_many(&self, values: Vec<T>) -> Result<usize> {
    let values = self.store.insert_many(values)?;
    let result = values.len();
    self
      .storage
      .save(values)
      .context(RdStoreErrorKind::DataSave)?;

    Ok(result)
  }

  pub fn find_many(&self, search: &T) -> Result<Vec<T>> {
    let result = self.store.find_many(search)?;
    Ok(result)
  }

  pub fn update_many(&self, search: &T, new_value: &T) -> Result<usize> {
    let values = self.store.update_many(search, new_value)?;
    let result = values.len();
    self
      .storage
      .save(values)
      .context(RdStoreErrorKind::DataSave)?;

    Ok(result)
  }

  pub fn delete_many(&self, search: &T) -> Result<usize> {
    let values = self.store.delete_many(search)?;
    let result = values.len();
    self
      .storage
      .save(values)
      .context(RdStoreErrorKind::DataSave)?;
    Ok(result)
  }
}
