use super::document::{Doc, Document};
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
  let guard = value.lock().unwrap();
  guard
}

pub fn find_value<'a, T, D>(store: &'a Read<D>, value: T) -> Vec<D>
where
  D: Doc<T>,
{
  let docs: Vec<D> = store
    .iter()
    .map(|(_id, doc)| doc.lock().unwrap())
    .filter(|doc| doc.get_status() != &Status::Deleted)
    .map(|doc| {
      //println!("{:?} VALUE", doc);
      doc.find_values(&value).to_owned()
    })
    .collect();

  docs
  //.filter(|(_id, doc)| doc.status != Status::Deleted)
  // .collect();
}
