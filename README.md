# RedStore (In development!)

`RedStore` is an embedded fast, lightweight, and secure in-memory data store with [persistance](#persistance) in different serde-compatible formats (json, ron, yaml)RedStore has an easy to use API for [finding](#find), [updating](#update) and [deleting](#finding) your data. 

## Quickstart

Add RedStore to your `Cargo.toml` specifing what serializer you want to use: 

```toml
[dependencies.RedStore]
version = "0.2.0"
features = ["json_ser"] # Json serialization / deserialization
features = ["ron_ser"] # Ron serialization / deserialization
features = ["yaml_ser"] # Yaml serialization / deserialization
```


```rust
use RedStore::{RonStore,JsonStore,YamlStore};

#[derive(Clone, Serialize, PartialEq, Deserialize)]
struct MyStruct {
  foo: String,
}

fn main() -> Result<()> {
  // RedStore with RON persistance for MyStruct documents
  let db = RonStore::new::<MyStruct>()?;

  let my_struct = MyStruct {
    foo: String::from("hello")
  };

  // Insert data
  let _id = db.insert_one(my_struct)?;
  // Find by ID
  let result: MyStruct = db.find_one(&id)?;
  // Find all records equal to my_struct
  let result = db.find(&my_struct)?;
  Ok(())
}

```

## API

- [Peristantce](#persistance)
- [Inserting data](#inserting-data)
- [Finding data](#finding-data)
- [Updating data](#updating-data)
- [Deleting data](#deleting-data)


### Persistance

RedStore's persistence uses an append-only format (AOF) so all write operations are added to to the end of the database file. The database is automatically compacted in just one line per object/data/record everytime you start the database in your application.

The API provides bulk-like write operations (insert, update, delete) for vectors of data that are faster to persist due to the AOF nature. Use them instead iterate over the `*_one()` methods you'll see on the API.


### Inserting Data

Redstore use UUID identifiers as unique ids. An UUid will be returned when you insert a record.

#### Insert one

```rust
#[derive(Clone, Serialize, PartialEq, Deserialize)]
struct MyStruct {
  foo: String,
}

let my_struct = MyStruct {
  foo: String::from("hello")
};

let id = store.insert_one(my_struct)?;
println!("{:?}", id);
// 94d69737-4b2e-4985-aaa1-e28bbff2e6d0
```

#### Insert 

If you want to insert a vector of data `insert()` is more suitable and faster to persists than iterate over `insert_one()` method due to the nature of the AOF persistance. 

```rust
let many = vec![MyStruct {
  foo: String::from("one"),
},
MyStruct {
  foo: String::from("two"),
}];

let inserted_ids = db.insert(many)?;
println!("{:?}", inserted_ids);
// [94d69737-4b2e-4985-aaa1-e28bbff2e6d0, 94d641737-4b2e-4985-aaa1-e28bbff2e6d0]
```

### Finding Data

There are two  ways to find your data. By it's ID or looking into the database what data matches your query.

#### Find one

Performs a search by ID.

```rust
let my_struct = MyStruct {
  foo: String::from("hello")
};

let id = db.insert_one(my_struct)?;
let result: MyStruct = db.find_one(&id)?;
println!("{:?}", result);
// MyStruct { foo: "hello" }
```

#### Find

Look into the database for data matching your query.

```rust
let one = MyStruct {
  foo: String::from("Hello"),
};

let two = MyStruct {
  foo: String::from("Hello"),
};

let three = MyStruct {
  foo: String::from("Bye"),
};


let many = vec![one.clone(), two.clone(), three.clone()];
let id = db.insert(many)?;
let result = db.find(&one)?;
println!("{:?}", result);
/* [
MyStruct { foo: "Hello" },
MyStruct { foo: "Hello" }
]*/
```

### Updating Data

Update data is pretty straightforward. You can update data

#### Update one

Update one record, using it's id as search param.

```rust
let my_struct = MyStruct {
  foo: String::from("hello")
};

let new_value = MyStruct {
  foo: String::from("bye"),
};

let id = db.insert_one(my_struct)?;
let updated = db.update_one(&id, new_value))?;
println!("{:?}", upated);
// true
```

#### Update 

You can update all data in the databas that matches your query param.

```rust
let search = MyStruct {
  foo: String::from("hello")
};

let new_value = MyStruct {
  foo: String::from("bye"),
};

let updated = store.update(&search, &new_value)?;
println!("{:?}", updated);
// 1
```

### Deleting Data

#### Delete one

Delete a record by it's ID.

```rust
let my_struct = MyStruct {
  foo: String::from("hello")
};

let id = db.insert_one(my_struct)?;
let deleted = db.delete_one(&id))?;
println!("{:?}", deleted);
// true
```

#### Delete 

Like in `update` method, this method will lookup into the database which data matches your query and then delete it.

```rust
let search = MyStruct {
  foo: String::from("hello")
};

let deleted = store.delete(&search)?;
println!("{:?}", updated);
// 1
```

## License

This library is licensed under either of

- Apache License, Version 2.0
	([LICENSE-APACHE](https://github.com/pmagaz/reddb/blob/master/LICENSE-APACHE)
	or
	[apache.org/licenses/LICENSE-2.0](https://apache.org/licenses/LICENSE-2.0))
- MIT license
	([LICENSE-MIT](https://github.com/pmagaz/reddb/blob/master/LICENSE-MIT)
	or
	[opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))

at your option.
