use super::document::Doc;
use super::status::Status;
use super::store::{Read, Write};

use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

pub fn insert<T, D>(store: &mut Write<D>, doc: D) -> Uuid
where
  D: Doc<T>,
{
  let _id = **&doc.get_id();
  let _result = store.insert(_id, Mutex::new(doc));
  _id
}

pub fn find_key<'a, T>(store: &'a Read<T>, id: &'a Uuid) -> MutexGuard<'a, T> {
  let value = store.get(&id).unwrap();
  let doc_guard = value.lock().unwrap();
  doc_guard
}

pub fn find_value<'a, T, D>(store: &'a Read<D>, value: T) -> Vec<D>
where
  D: Doc<T>,
{
  let docs: Vec<D> = store
    .iter()
    .map(|(_id, doc)| doc.lock().unwrap())
    .filter(|doc| doc.get_status() != &Status::Deleted)
    .filter(|doc| doc.find_in_values(&value))
    .map(|doc| doc.to_owned())
    .collect();

  docs
}
