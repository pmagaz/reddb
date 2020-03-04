use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
extern crate reddb;
use reddb::RedDb;

fn main() {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  struct MyStruct {
    leches: String,
  };

  // let db = RedDb::<MyStruct>::new();
  // let id = db.insert(MyStruct {
  //   leches: String::from("hola"),
  // });

  // let result = db.find_all(MyStruct {
  //   leches: String::from("hola"),
  // });
  // let result = db.find_one(&id);
  // println!("FIND_ONE {:?}", result);

  let db2 = RedDb::<Value>::new();
  let id = db2.insert(json!({ "leches": true}));
  let id = db2.insert(json!({ "leches": true, "boo": 12}));
  let id2 = db2.insert(json!({ "leches": false}));
  let result = db2.find_one(&id);
  println!("FIND_ONE {:?}", result);
  db2.insert(json!({"name":"record1", "leches": 11}));
  let result = db2.find_all(json!({"name":"record1", "leches": 11}));
  println!("FIND ALL {:?}", result);
  let id = db2.delete_one(&id2);
  let result = db2.find_all(json!({ "leches": false}));
  println!("FIND DELETED ONE {:?}", result);
  let id = db2.delete_all(json!({ "leches": true}));
  let result = db2.find_all(json!({ "leches": false}));
  println!("FIND DELETED ALL {:?}", result);
  let result = db2.find_all(json!({ "leches": true, "boo": 12}));
  println!("FIND ALL {:?}", result);
  let id3 = db2.insert(json!({ "record": true, "foo": 11}));
  let updated = db2.update_one(&id3, json!({ "record": false, "foo": 22}));
  let result = db2.find_one(&id3);
  println!("UPDATED ONE {:?}", result);
}
