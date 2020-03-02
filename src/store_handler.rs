use super::record::{Record, RedDbRecord};
use super::status::Status;
use super::store::{Read, Write};

use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

pub fn insert<T, R>(store: &mut Write<R>, record: R) -> Uuid
where
  R: Record<T>,
{
  let _id = **&record.get_id();
  let _result = store.insert(_id, Mutex::new(record));
  _id
}

pub fn find_key<'a, T>(store: &'a Read<T>, id: &'a Uuid) -> MutexGuard<'a, T> {
  let value = store.get(&id).unwrap();
  let guard = value.lock().unwrap();
  guard
}
