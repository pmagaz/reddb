use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
extern crate reddb;
use reddb::RedDb;

fn main() {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  struct MyStruct {
    leches: String,
  };

  let db = RedDb::<MyStruct>::new();
  let id = db.insert(MyStruct {
    leches: String::from("hola"),
  });
  let result = db.find(MyStruct {
    leches: String::from("hola"),
  });
  println!("FIND Struct {:?}", result);
  // let result = db.find_one(&id);
  // println!("FIND_ONE {:?}", result);

  let db2 = RedDb::<Value>::new();
  //let id = db.insert(json!({ "leches": true}));
  //let result = db.find_one(&id);
  //println!("FIND_ONE {:?}", result);
  db2.insert(json!({"name":"record1", "leches": 11}));
  let result = db2.find(json!({"name":"record1", "leches": 11}));
  println!("FIND JSON{:?}", result);
}
