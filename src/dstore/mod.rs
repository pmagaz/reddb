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
pub struct DStore {
    handler: Handler,
    store: RwLock<HashMap<String, String>>,
}

impl DStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore> {
        let mut buf = Vec::new();
        let handler = Handler::new(path)?;
        handler
            .file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();

        let map: HashMap<String, String> = HashMap::new();
        Ok(Self {
            handler: handler,
            store: RwLock::new(map),
        })
    }

    pub fn put(&mut self, key: String, value: String) -> &mut DStore {
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
        //file.set_len(0);
        file.write(&buf);
        file.sync_all()?;
        Ok(())
    }
}
