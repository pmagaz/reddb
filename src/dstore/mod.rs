use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::Path;
use std::result;
use std::sync::{RwLock, RwLockWriteGuard};

//use serde::{Serialize, Deserialize};
use bincode::{deserialize as bin_deserialize, serialize as bin_serialize};

mod error;
mod handler;
use handler::Handler;
pub type Result<T> = result::Result<T, error::DStoreError>;

#[derive(Debug)]
pub struct DStore<T: Eq + Hash + Serialize + DeserializeOwned> {
    handler: Handler,
    store: RwLock<HashMap<T, T>>,
}

impl<T: Eq + Hash + Serialize + DeserializeOwned> DStore<T> {
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
        //print!("bufff{:?}", buf);

        let map: HashMap<T, T> = if (!buf.is_empty()) {
            //serde_json::from_value(&buf).unwrap()
            bin_deserialize(&buf).unwrap()
        } else {
            HashMap::new()
        };
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

    pub fn put(&mut self, key: T, value: T) -> &mut DStore<T> {
        {
            let mut lock = self.store.write().unwrap();
            lock.insert(key, value);
        }
        self
    }

    pub fn persist(&mut self) -> Result<()> {
        let map = self.store.read()?;
        let mut file = self.handler.file.lock()?;
        let buf = bin_serialize(&*map).unwrap();
        file.write(&buf);
        file.sync_all()?;
        Ok(())
    }
}
