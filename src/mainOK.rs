// use std::collections::HashMap;
// use std::io::{Error, ErrorKind};
// use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard};
// use uuid::Uuid;

// pub type ReadGuard<'a, T> = RwLockReadGuard<'a, T>;

// #[derive(Clone, Default)]
// pub struct Store {
//   pub store: Arc<RwLock<HashMap<Uuid, Mutex<String>>>>,
// }

// impl Store {
//   pub fn new() -> Self {
//     let mut map_store = HashMap::new();
//     map_store.insert(Uuid::new_v4(), Mutex::new(String::from("Hello")));

//     Self {
//       store: Arc::new(RwLock::new(map_store)),
//     }
//   }

//   pub fn find_by_id<'a>(
//     &self,
//     map: &'a RwLockReadGuard<HashMap<Uuid, Mutex<String>>>,
//     id: &'a str,
//   ) -> &'a String {
//     let uuid = Uuid::parse_str(id).unwrap();
//     //let map = self.store.read().unwrap();
//     let doc = map
//       .get(&uuid)
//       .ok_or_else(|| Error::new(ErrorKind::NotFound, "Not found"))
//       .unwrap()
//       .lock()
//       .unwrap();

//     //let guard = doc.lock().unwrap();
//     &*doc
//   }

//   fn leches<'a>(
//     &self,
//     map: &'a RwLockReadGuard<HashMap<Uuid, Mutex<String>>>,
//     uuid: Uuid,
//   ) -> &'a String {
//     let doc = map
//       .get(&uuid)
//       .ok_or_else(|| Error::new(ErrorKind::NotFound, "Not found"))
//       .unwrap();

//     let guard = doc.lock().unwrap();
//     &*guard
//   }
// }

// fn main() {
//   let store = Store::new();
//   let map = store.store.read().unwrap();

//   let result = store.find_by_id(&map, "e7cdef61-d09d-420a-9a3d-e485c056c6aa");
// }

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

#[derive(Debug)]
pub struct Document {
  pub data: Value,
}

pub struct Store<T> {
  pub store: RwLock<HashMap<Uuid, T>>,
}

impl<T> Store<T> {
  pub fn new() -> Self {
    let map: HashMap<Uuid, T> = HashMap::new();
    Self {
      store: RwLock::new(map),
    }
  }

  pub fn find_by_id<'a>(
    &self,
    map: &'a RwLockReadGuard<HashMap<Uuid, T>>,
    id: &Uuid,
  ) -> Option<&'a T> {
    map.get(&id)
  }

  pub fn insert<'a>(&self, key: Uuid, val: T) -> Option<T> {
    let mut write = self.store.write().unwrap();
    write.insert(key, val)
  }

  // pub fn find_one<'a>(&self, id: &'a Uuid) -> Option<&'a T> {
  //   let read = self.store.read().unwrap();
  //   let document = self.find_by_id(&read, &id);

  //   // let result = match document {
  //   //   // The division was valid
  //   //   Some(x) => x,
  //   //   // The division was invalid
  //   //   None => println!("Cannot divide by 0"),
  //   // };
  //   document
  // }
}

// pub fn find_by_id<'a, T>(map: &'a RwLockReadGuard<HashMap<Uuid, T>>, id: &Uuid) -> Option<&'a T> {
//   map.get(&id)
// }

fn main() {
  //let map: RwLock<HashMap<Uuid, Mutex<Document>>> = RwLock::new(HashMap::new());
  let store = Store::<Mutex<Document>>::new();
  let id = Uuid::new_v4();
  let doc = Document {
    data: json!({"name":"Peter"}),
  };
  //let read = store.store.read().unwrap();
  //let mut insert = store.store.write().unwrap();
  store.insert(id, Mutex::new(doc));
  let read = store.store.read().unwrap();
  //let document = store.find_one(&id);
  let document = store.find_by_id(&read, &id);
  //println!("{:?}", map);
  println!("{:?}", document);
}
