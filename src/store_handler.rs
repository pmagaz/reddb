use super::document::Doc;
use super::status::Status;
use super::store::{Read, Write};

use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

#[derive(Debug)]
pub struct Handler;

impl Handler {
  pub fn insert<T, D>(&self, store: &mut Write<D>, doc: D) -> Uuid
  where
    D: Doc<T>,
  {
    let _id = **&doc.get_id();
    let _result = store.insert(_id, Mutex::new(doc));
    _id
  }

  pub fn find_key<'a, D>(&self, store: &'a Read<D>, id: &'a Uuid) -> MutexGuard<'a, D> {
    let value = store.get(&id).unwrap();
    let doc = value.lock().unwrap();
    doc
  }

  pub fn update_key<'a, T, D>(
    &self,
    store: &'a mut Write<D>,
    id: &'a Uuid,
    newValue: T,
  ) -> MutexGuard<'a, D>
  where
    D: Doc<T>,
  {
    let mut value = store.get_mut(&id).unwrap();
    let mut doc = value.lock().unwrap();
    doc.set_data(newValue);
    doc.set_status(Status::Updated);
    doc
    //*value = doc
  }

  pub fn delete_key<'a, T, D>(&self, store: &mut Write<D>, id: &'a Uuid) -> D
  where
    D: Doc<T>,
  {
    let result = store.remove(id).unwrap();
    let mut doc = result.lock().unwrap();
    doc.set_status(Status::Deleted);
    doc.to_owned()
  }

  pub fn find_from_value<'a, T, D>(&self, store: &'a Read<D>, value: T) -> Vec<D>
  where
    D: Doc<T>,
  {
    let docs: Vec<D> = store
      .iter()
      .map(|(_id, doc)| doc.lock().unwrap())
      .filter(|doc| doc.get_status() != &Status::Deleted)
      .filter(|doc| doc.match_values(&value))
      .map(|doc| doc.to_owned())
      .collect();

    docs
  }

  pub fn update_from_value<'a, T, D>(&self, store: &mut Write<D>, value: T) -> Vec<D>
  where
    D: Doc<T>,
  {
    let docs: Vec<D> = store
      .iter_mut()
      .map(|(_id, doc)| doc.lock().unwrap())
      .filter(|doc| doc.get_status() != &Status::Deleted)
      .filter(|doc| doc.match_values(&value))
      .map(|doc| doc.to_owned())
      .collect();

    docs
  }
}
