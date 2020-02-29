use serde_json::{json, Value};

extern crate reddb;
use reddb::RedDb;

fn main() {
  let db = RedDb::<Value>::new();
  let id = db.insert(json!({ "leches": true}));
  let result = db.find_one(&id);
  println!("{:?}", result);
}
