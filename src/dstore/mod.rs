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
use store::{Document, Store};
use uuid::Uuid;

pub type Result<T> = result::Result<T, error::DStoreError>;

#[derive(Debug)]
pub struct DStore {
    store: Store,
    db_storage: Handler,
    opt_storage: Handler,
}

impl DStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore> {
        let mut buf = Vec::new();
        let db_storage = Handler::new(path)?;
        let opt_storage = Handler::new(".opt.aof")?;
        db_storage.read_content(&mut buf);
        let store = Store::new(buf.lines())?;
        println!("[DStore] Ok");

        Ok(Self {
            store: store,
            db_storage: db_storage,
            opt_storage: opt_storage,
        })
    }

    pub fn find_id(&self, query: &Value) -> Result<Value> {
        Ok(self.store.find_id(&query)?)
    }

    pub fn find(&self, query: &Value) -> Result<Value> {
        Ok(self.store.find(&query)?)
    }

    pub fn insert(&mut self, query: Value) -> Result<Value> {
        Ok(self.store.insert(query)?)
    }

    pub fn update(&mut self, query: Value, new_value: Value) -> Result<usize> {
        let documents = self.store.update(query, new_value)?;
        Ok(documents.len())
    }

    pub fn delete(&mut self, query: Value) -> Result<usize> {
        let documents = self.store.delete(query)?;
        self.log_operation(&documents)?;
        Ok(documents.len())
    }

    pub fn log_operation(&self, documents: &Vec<(Uuid, Document)>) -> Result<()> {
        let mut opt_storage = self.opt_storage.file.lock()?;
        let operation_log = self.store.format_operation(documents);
        opt_storage.seek(SeekFrom::End(0))?;
        opt_storage.write_all(&operation_log)?;
        opt_storage.sync_all()?;
        Ok(())
    }

    pub fn get(&mut self) -> Result<()> {
        Ok(self.store.get()?)
    }

    pub fn persist(&mut self) -> Result<()> {
        let mut file = self.db_storage.file.lock()?;
        let docs_to_save = self.store.format_jsondocs();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&docs_to_save)?;
        file.sync_all()?;
        Ok(())
    }
}
