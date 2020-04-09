use super::deserializer::DeSerializer;
use super::operation::Operation;
use super::record::{Empty, Record};
use super::storage::Storage;
use core::fmt::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::{BufRead, Seek, SeekFrom, Write};
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::result;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

type ByteString = Vec<u8>;
type WriteOps<T> = Vec<(Uuid, T, Operation)>;
pub type Result<T> = result::Result<T, std::io::Error>;

pub type RDHM = HashMap<Uuid, Mutex<ByteString>>;

#[derive(Debug)]
pub struct Store<DS> {
  pub store: RwLock<RDHM>,
  pub storage: Storage,
  pub serializer: DS,
}

impl<'a, DS> Store<DS>
where
  for<'de> DS: DeSerializer<'de> + Debug + Clone,
{
  pub fn new<S, P>(path: P, des: S) -> Self
  where
    P: AsRef<Path>,
    for<'de> S: DeSerializer<'de> + Debug + Clone,
  {
    let mut hm: RDHM = HashMap::new();
    //des.serializer(val: &T)

    let storage = Storage::new(path).unwrap();
    let mut buf = Vec::new();
    storage.read_content(&mut buf);

    for (_index, content) in buf.lines().enumerate() {
      let line = content.unwrap();
      let leches = line.as_bytes();
      //let record : Record = des.from_str(line);
      //println!("{:}", des.from(leches));
      //des.serializer(line);
    }
    //DS::deserializer(line);
    // for (_index, line) in buf.lines().enumerate() {
    // let content = &line.unwrap();
    // let json_doc: JsonDocument = serde_json::from_str(content)?;
    // let _id = match &json_doc._id.as_str() {
    //   Some(_id) => Uuid::parse_str(_id).unwrap(),
    //   None => panic!("ERR: Wrong Uuid format!"),
    // };
    // let doc = Document {
    //   store: json_doc.store,
    //   status: Status::Saved,
    // };
    // map.insert(_id, doc);
    //}

    // let storage = OpenOptions::new()
    //   .read(true)
    //   .append(true)
    //   .create(true)
    //   .open(&path)?;

    Self {
      store: RwLock::new(hm),
      storage: storage,
      serializer: DS::default(),
    }
  }

  // pub fn load(&self) -> Result<Self> {
  //   //let storage = Storage::new(path).unwrap();
  //   let mut buf = Vec::new();
  //   self.storage.read_content(&mut buf);

  //   for (_index, content) in buf.lines().enumerate() {
  //     let line = &content.unwrap();
  //     println!("{:}", line);
  //     //DS::deserializer(line);
  //     //dese
  //     //DS::deserializer(line);
  //     //let record: Record = self.serializer.deserializer(line);
  //     // let json_doc: JsonDocument = serde_json::from_str(content)?;
  //     // let _id = match &json_doc._id.as_str() {
  //     //   Some(_id) => Uuid::parse_str(_id).unwrap(),
  //     //   None => panic!("ERR: Wrong Uuid format!"),
  //     // };
  //     // let doc = Document {
  //     //   store: json_doc.store,
  //     //   status: Status::Saved,
  //     // };
  //     // map.insert(_id, doc);
  //   }
  //   Ok(Self {
  //     store: self.store,
  //     storage: self.storage,
  //     serializer: self.serializer,
  //   })
  // }

  pub fn to_read(&'a self) -> RwLockReadGuard<'a, RDHM> {
    let read = self.store.read().unwrap();
    read
  }

  pub fn to_write(&'a self) -> RwLockWriteGuard<'a, RDHM> {
    let write = self.store.write().unwrap();
    write
  }

  pub fn insert_key(&self, id: Uuid, data: ByteString) -> Option<Mutex<ByteString>> {
    let mut store = self.to_write();
    store.insert(id, Mutex::new(data))
  }

  pub fn delete_key(&self, id: &Uuid) -> Mutex<ByteString> {
    let mut store = self.to_write();
    let result = store.remove(id).unwrap();
    result
  }

  pub fn find_keys<T>(&self, search: &T) -> Vec<Uuid>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let store = self.to_read();
    //self::DeSerializer::deserializer()
    let serialized = self.serializer.serializer(search);
    let docs: Vec<Uuid> = store
      .iter()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(id, _value)| *id)
      .collect();
    docs
  }

  pub fn insert_one<T>(&self, value: &T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let id = Uuid::new_v4();
    let data = self.serializer.serializer(value);
    let _result = self.insert_key(id, data);
    id
  }

  pub fn find_one<T>(&self, id: &Uuid) -> T
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let store = self.to_read();
    let value = store.get(&id).unwrap();
    let guard = value.lock().unwrap();
    self.serializer.deserializer(&*guard)
  }

  pub fn update_one<T>(&'a self, id: &Uuid, new_value: &T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let mut store = self.to_write();
    let value = store.get_mut(&id).unwrap();
    let mut guard = value.lock().unwrap();
    *guard = self.serializer.serializer(new_value);
    id.to_owned()
  }

  pub fn delete_one(&self, id: &Uuid) -> Uuid {
    let mut store = self.to_write();
    let _result = store.remove(id).unwrap();
    id.to_owned()
  }

  pub fn insert<T>(&self, values: Vec<T>) -> WriteOps<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let docs: WriteOps<T> = values
      .iter()
      .map(|value| {
        let id = Uuid::new_v4();
        let serialized = self.serializer.serializer(value);
        let _result = self.insert_key(id, serialized);
        (id, value.clone(), Operation::Insert)
      })
      .collect();
    docs
  }

  pub fn find<T>(&self, search: &T) -> Vec<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let store = self.to_read();
    let serialized = self.serializer.serializer(search);
    let docs: Vec<T> = store
      .iter()
      .map(|(_id, value)| value.lock().unwrap())
      .filter(|value| **value == serialized)
      .map(|value| self.serializer.deserializer(&*value))
      .collect();
    docs
  }

  pub fn update<T>(&self, search: &T, new_value: &T) -> WriteOps<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let mut store = self.to_write();
    let serialized = self.serializer.serializer(search);

    let docs: WriteOps<T> = store
      .iter_mut()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(_id, mut value)| {
        *value = self.serializer.serializer(new_value);
        (*_id, new_value.clone(), Operation::Update)
      })
      .collect();
    docs
  }

  pub fn delete<T>(&self, search: &T) -> WriteOps<Empty>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let keys = self.find_keys(search);
    let docs: WriteOps<Empty> = keys
      .iter()
      .map(|id| {
        self.delete_key(id);
        (*id, Empty, Operation::Delete)
      })
      .collect();
    docs
  }

  pub fn persist<T>(&self, docs: Vec<(Uuid, T, Operation)>)
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display,
  {
    let serialized: ByteString = docs
      .into_iter()
      .map(|(id, value, status)| Record::new(id, value, status))
      .flat_map(|record| self.serializer.serializer(&record))
      .collect();
    &self.storage.write(&serialized);
  }

  pub fn persist_one<T>(&self, id: Uuid, data: T, status: Operation)
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display,
  {
    let record = Record::new(id, data, status);
    let serialized = self.serializer.serializer(&record);
    &self.storage.write(&serialized);
  }
}
