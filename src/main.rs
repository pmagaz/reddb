use serde::{Deserialize, Serialize};
use std::fmt;
extern crate reddb;
use reddb::{JsonSerializer, RedDb};

fn main() {
  #[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
  struct MyStruct {
    foo: String,
  }

  impl fmt::Display for MyStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "({})", self)
    }
  }

  //STRUCTS
  println!("STRUCTS");
  let query = MyStruct {
    foo: String::from("hola"),
  };
  let new_value = MyStruct {
    foo: String::from("new Value"),
  };
  let new_value2 = MyStruct {
    foo: String::from("new Value"),
  };
  let new_value3 = MyStruct {
    foo: String::from("new Value3"),
  };
  let db = RedDb::<JsonSerializer>::new();
  let _id = db.insert_one(query.clone());
  let _id = db.insert_one(String::from("hola"));
  let id = db.insert_one(MyStruct {
    foo: String::from("holaa"),
  });
  let result = db.find_one::<MyStruct>(&id);
  println!("FIND_ONE {:?}", result);
  let result = db.find_all(&query);
  println!("FIND ALL {:?}", result);
  let result = db.update_one::<MyStruct>(&_id, new_value);
  println!("FIND ONE UPDATED {:?}", result);

  let result = db.delete_one(&_id);
  println!("FIND ONE DELETED {:?}", result);
  let result = db.update_all(&query, new_value2);
  println!("UPDATE ALL {:?}", result);

  let result = db.update_all(&query, new_value3.clone());
  println!("UPDATE ALL {:?}", result);

  let another = MyStruct {
    foo: String::from("22"),
  };
  let id = db.insert_one(another.clone());
  let id = db.insert_one(another.clone());
  let result = db.delete_all(&another);
  println!("DELETE ALL {:?}", result);

  // println!("JSON STRINGS");
  // let db = RedDb::<String, JsonSerializer>::new();
  // let query = r#"
  //       {
  //           "leches": true,
  //           "boo": 12,
  //       }"#;

  // let query2 = r#"
  //       {
  //           "leches": false,
  //           "boo": 12,
  //       }"#;

  // let id = db.insert(query.to_owned());
  // println!("INSERT {:?}", &id);
  // let result = db.find_one(&id);
  // println!("FIND_ONE {:?}", result);
  // let result = db.find_all(&query2.to_owned());
  // println!("FIND ALL {:?}", result);
  //JSON VALUES
  //println!("JSON VALUES");

  // let db2 = RedDb::<Value, JsonSerializer>::new();
  // let _id = db2.insert(json!({ "leches": true}));
  // let id = db2.insert(json!({ "leches": true, "boo": 12}));
  // let id2 = db2.insert(json!({ "leches": false}));
  // let result = db2.find_one(&id);
  // println!("FIND_ONE {:?}", result);
  // db2.insert(json!({"name":"record1", "leches": 11}));
  // let result = db2.find_all(json!({"name":"record1", "leches": 11}));
  // println!("FIND ALL {:?}", result);
  // db2.delete_one(&id2);
  // let result = db2.find_all(json!({ "leches": false}));
  // println!("FIND DELETED ONE {:?}", result);
  // db2.delete_all(json!({ "leches": true}));
  // let result = db2.find_all(json!({ "leches": false}));
  // println!("FIND DELETED ALL {:?}", result);
  // let result = db2.find_all(json!({ "leches": true, "boo": 12}));
  // println!("FIND ALL {:?}", result);
  // let id3 = db2.insert(json!({ "record": true, "foo": 11}));
  // db2.update_one(&id3, json!({ "record": "updateeeed"}));
  // let result = db2.find_one(&id3);
  // println!("UPDATED ONE {:?}", result);
  // db2.update_all(
  //   json!({ "record": "updateeeed"}),
  //   json!({ "record": "updated!"}),
  // );
  // let result = db2.find_all(json!({ "record": "updated!"}));
  // println!("UPDATED ALL {:?}", result);
}
