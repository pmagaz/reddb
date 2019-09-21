use serde::de::DeserializeOwned;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;

use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::Path;
use std::result;
use std::sync::{RwLock, RwLockWriteGuard};
//use serde::{Serialize, Deserialize};
//use bincode::{deserialize as bin_deserialize, serialize as bin_serialize};

mod error;
mod handler;
use handler::Handler;
pub type Result<T> = result::Result<T, error::DStoreError>;

#[derive(Debug)]
pub struct DStore<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    handler: Handler,
    store: RwLock<HashMap<T, Document>>,
}

#[derive(Debug, Deserialize)]
struct Collection {
    name: String,
    data: HashMap<String, Document>,
    createdAt: String,
    updatedAt: String,
}
#[derive(Debug, Deserialize)]
struct Document {
    id: String,
    user: String,
}

impl<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> DStore<T> {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore<T>> {
        let mut buf = Vec::new();
        let handler = Handler::new(path)?;
        //print!("bufff{:?}", buf);
        handler
            .file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();

        let mut map = HashMap::new();
        let collection: Collection = serde_json::from_slice(&buf).unwrap();
        //println!("aaaaaa{:?}", collection.docs);
        let config: HashMap<String, Document> = collection.data; //serde_json::from_slice(&buf).unwrap();
        println!("aaaaaa{:?}", config.len());
        //let config: HashMap<String, String> = serde_json::from_slice(&buf).unwrap();

        //serde_json::Map::
        //let map3: HashMap<String, Document> = serde_json::from_str(collection);
        //let mut map2 = HashMap::new();
        //let map3: HashMap<T, Document> = serde_json::from_slice(&buf).unwrap();

        //let mut map = HashMap::new();
        // for doc in collection.docs {
        //     // map.insert(doc.id,doc)

        // }
        Ok(Self {
            handler: handler,
            store: RwLock::new(map),
        })
    }

    pub fn get(&self) -> Result<()> {
        let data = self.store.read()?;
        // println!("aaaaaaa{}", data);
        Ok(())
    }

    pub fn put(&mut self, value: T, key: Option<u32>) -> &mut DStore<T> {
        {
            let key = match key {
                Some(key) => key,
                _ => 1,
            };
            let mut lock = self.store.write().unwrap();
            //lock.insert(key, value);
        }
        self
    }

    pub fn persist(&mut self) -> Result<()> {
        let map = self.store.read()?;
        let mut file = self.handler.file.lock()?;
        //let buf = serialize(&*map).unwrap();
        //file.write(&buf);
        file.sync_all()?;
        Ok(())
    }
}
