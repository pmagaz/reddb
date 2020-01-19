use serde_json::Value;
use std::io::{BufRead, Seek, SeekFrom, Write};
use std::path::Path;
use std::result;

mod error;
mod handler;
mod json;
mod status;
mod store;

use handler::Handler;
use store::Store;

pub type Result<T> = result::Result<T, error::DStoreError>;

#[derive(Debug)]
pub struct DStore {
    store: Store,
    handler: Handler,
}

impl DStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore> {
        let mut buf = Vec::new();
        let handler = Handler::new(path)?;
        handler.read_content(&mut buf);
        let store = Store::new(buf.lines())?;
        println!("[DStore] Ok");

        Ok(Self {
            store: store,
            handler: handler,
        })
    }
    //TODO data to be json
    pub fn insert(&mut self, data: String) -> Result<Value> {
        Ok(self.store.insert(data)?)
    }

    pub fn delete(&mut self, data: String) -> Result<Value> {
        Ok(self.store.delete(data)?)
    }

    pub fn find_by_id(&self, id: &Value) -> Result<Value> {
        Ok(self.store.find_by_id(id)?)
    }

    pub fn find(&self, data: String) -> Result<Value> {
        Ok(self.store.find(data)?)
    }

    pub fn get(&self) {
        self.store.get().unwrap()
    }

    pub fn persist(&mut self) -> Result<()> {
        let mut file = self.handler.file.lock()?;
        let docs_to_save = self.store.format_jsondocs();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&docs_to_save)?;
        file.sync_all()?;
        Ok(())
    }
}
