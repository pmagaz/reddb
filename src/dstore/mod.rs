use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, Read, Write};
use std::path::Path;
use std::result;
use std::hash::Hash;
use std::sync::{Mutex, RwLock, RwLockWriteGuard};
//use serde::{Serialize, Deserialize};

mod error;
mod handler;
use handler::Handler;
pub type Result<T> = result::Result<T, error::DStoreError>;

#[derive(Debug)]
struct Document {
    data: String,
}

#[derive(Debug)]
pub struct DStore<T: String + Eq + Hash> {
    handler: Handler,
    store: RwLock<HashMap<T, T>>,
}

impl<T: Eq + Hash> DStore<T> {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore<T>> {
        let mut handler = Handler::new(path)?;
        let mut buf = Vec::new();
        let content = handler.file.read_to_end(&mut buf)?;
        let mut map: HashMap<T, T> = HashMap::new();
        Ok(Self {
            handler: handler,
            store: RwLock::new(map),
        })
    }

    pub fn put<K,V>(&self, key: K, value: V) -> Result<()> {
        let mut lock = self.store.write()?;
        lock.insert(key, value);
        Ok(())
    }

    pub fn persist(&mut self) -> Result<()> {
        self.handler.file.write_all(b"hola")?;
        self.handler.file.sync_all()?;
        Ok(())
    }
}
