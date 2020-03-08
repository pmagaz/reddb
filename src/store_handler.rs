use super::document::{Doc, Document};
use super::json_ser::DeSerializer;
use super::status::Status;
use super::store::{Read, Write};
use serde::de::Deserialize;
use serde::Serialize;
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

#[derive(Debug)]
pub struct Handler<S> {
  pub serializer: S,
}

impl<S> Handler<S> {
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
    new_value: T,
  ) -> MutexGuard<'a, D>
  where
    D: Doc<T>,
  {
    let value = store.get_mut(&id).unwrap();
    let mut doc = value.lock().unwrap();
    doc.set_data(new_value);
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

  pub fn find_from_value<'a, T, D, DS>(&self, store: &'a Read<D>, query: T) -> Vec<D>
  where
    D: Doc<T> + Serialize + Deserialize<'a>,
    DS: DeSerializer<'a, D>,
  {
    let docs: Vec<D> = store
      .iter()
      .map(|(_id, doc)| doc.lock().unwrap())
      .filter(|doc| doc.get_status() != &Status::Deleted)
      .filter(|doc| doc.find_content(&query))
      .map(|doc| doc.to_owned())
      .collect();

    docs
  }

  pub fn update_from_value<'a, T, D, DS>(
    &self,
    store: &mut Write<D>,
    serializer: &S,
    query: T,
    new_value: T,
  ) -> Vec<D>
  where
    D: Doc<T> + Serialize + Deserialize<'a>,
    S: DeSerializer<'a, D>,
  {
    let docs: Vec<D> = store
      .iter_mut()
      .map(|(_id, doc)| doc.lock().unwrap())
      .filter(|doc| doc.get_status() != &Status::Deleted)
      .map(|doc| {
        let id = doc.get_id();
        let content = doc.update_content(&query, &new_value);
        let leches = serializer.serialize(&*doc);
        println!("{:?}", leches);
        doc
      })
      .map(|doc| doc.to_owned())
      .collect();

    docs
  }
}
