# RedDb

[![Actions Status](https://github.com/pmagaz/reddb/workflows/build/badge.svg)](https://github.com/pmagaz/reddb/actions) [![Crates.io](https://img.shields.io/crates/v/reddb)](https://crates.io/crates/reddb)

`RedDb` is an async, fast, lightweight and embedded in-memory document database with [persistance](#persistance) in different serde-compatible formats (ron and json at the moment and bindcode and cbor soon). RedDb uses [Tokio](https://github.com/tokio-rs/tokio) fort its easy to use async API for [inserting](#inserting-data), [finding](#finding-data), [updating](#updating-data) and [deleting](#deleting-data) data.

## Quickstart

Add RedDb to your `Cargo.toml` specifing what serializer you want to use:

```toml
[dependencies.RedDb]
version = "0.2.3"
features = ["ron_ser"] # Ron serialization / deserialization
features = ["json_ser"] # Json serialization / deserialization

```

```rust
use reddb::{Document, RonDb};

#[derive(Clone, Serialize, PartialEq, Deserialize)]
struct MyStruct {
  foo: String,
}

#[tokio::main]
async fn main() -> Result<()> {
  // RedDb with RON persistance for MyStruct structs
  let db = RonDb::new::<MyStruct>("my.db").unwrap();
  let my_struct = MyStruct {
    foo: String::from("hello")
  };

  // Insert data
  let doc = db.insert_one(my_struct).await?;
  // Find by id
  let my_doc: Document<MyStruct> = db.find_one(&doc._id).await?;
  // Find all records equal to my_struct
  let my_docs : Vec<Document<MyStruct>> = db.find(&my_struct).await?;
  Ok(())
}
```

## Why

RedDb is the migration of a side project originally written in NodeJs that was designed to store objects in memory (with hd persistance) and do searchs on them.

## When

If you are looking for a classic Key/Value storage you will find better options since RedDb is not a Key/Value (RedDb uses autogeneratd Uuids). You can store any kind of [data](#Data) since data will be handled as a [generic](#Data) but RedDb was designed to store Objects/Structs and peform basic search operations in those Structs. Said that, if you are looking for a lightweight and easy to use in-memory data store with [persistance](#persistance), RedDb could be a good choice!

## API

- [Data](#data)
- [Peristantce](#persistance)
- [Inserting data](#inserting-data)
- [Finding data](#finding-data)
- [Updating data](#updating-data)
- [Deleting data](#deleting-data)

### Data

Data is serialized and deserialized in different serde-compatible formats (json, ron, yaml) and wrapped into the Document struct as follows:

```rust
pub struct Document<T> {
  pub _id: Uuid,
  pub data: T,
  pub _st: Status,
}
```

Since data field is a generic you can store any kind of data you want. As you will see on the API, Document&lt;T> is the default return type for most operations.

### Persistance

RedDb's persistence uses an append-only format (AOF) so all write operations (Insert, Update, Delete) are added to to the end of the database file. The database is automatically compacted in just one line per object/record everytime you start the database in your application.

The API provides bulk-like write operations (insert, update and delete) for vectors of data that are faster to persist due to hd sync operations. Use them instead iterate over the `*_one()` methods you'll see on the API.

### Inserting Data

Insert data is pretty straightforward. If you want to insert just one document use insert_one method:

#### Insert one

```rust
#[derive(Clone, Serialize, PartialEq, Deserialize)]
struct MyStruct {
  foo: String,
}

let my_struct = MyStruct {
  foo: String::from("hello")
};

let doc: Document<TestStruct> = store.insert_one(my_struct).await?;
println!("{:?}", doc._id);
// 94d69737-4b2e-4985-aaa1-e28bbff2e6d0
```

#### Insert

If you want to insert a vector of data `insert()` is more suitable and faster to persists than iterate over `insert_one()` method due to the nature of the AOF persistance.

```rust
let my_docs = vec![MyStruct {
  foo: String::from("one"),
},
MyStruct {
  foo: String::from("two"),
}];

let docs: Vec<Document<MyStruct>> = db.insert(my_docs).await?;
```

### Finding Data

There are two ways to find your data. By it's id or looking into the database what data matches your query.

#### Find one

Performs a search by id.

```rust
let my_struct = MyStruct {
  foo: String::from("hello")
};

let inserted_doc = db.insert_one(my_struct).await?;
let doc: Document<MyStruct> = db.find_one(&inserted_doc._id).await?;
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
let inserted_doc : Document<MyStruct> = db.insert(many).await?;
let docs: Vec<Document<MyStruct>> = db.find(&one).await?;
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

let inserted_doc = db.insert_one(my_struct).await?;
let updated: bool = db.update_one(&inserted_doc._id, new_value)).await?;
```

#### Update

You can update all data in the databas that matches your query param. Update will return the number of updated documents.

```rust
let search = MyStruct {
  foo: String::from("hello")
};

let new_value = MyStruct {
  foo: String::from("bye"),
};

let updated: usize = store.update(&search, &new_value).await?;
```

### Deleting Data

#### Delete one

Delete a record by it's id.

```rust
let my_struct = MyStruct {
  foo: String::from("hello")
};

let doc = db.insert_one(my_struct).await?;
let deleted : bool = db.delete_one(&doc._id)).await?;
```

#### Delete

Like in `update` method, this method will lookup into the database which data matches your query and then delete it.

```rust
let search = MyStruct {
  foo: String::from("hello")
};

let deleted = store.delete(&search).await?;
println!("{:?}", updated);
// 1
```

## License

This library is licensed under

- MIT license
  ([LICENSE-MIT](https://github.com/pmagaz/reddb/blob/master/LICENSE-MIT)
  or
  [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))
