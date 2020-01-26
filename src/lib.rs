use serde_json::Value;
use std::io::{BufRead, Seek, SeekFrom, Write};
use std::path::Path;
use std::result;

#[macro_use]
extern crate quick_error;

mod error;
mod json;
mod query;
mod status;
mod storage;
mod store;
use query::Query;
use storage::Storage;
use store::{Document, Store};
use uuid::Uuid;

pub type Result<T> = result::Result<T, error::RedDbError>;

#[derive(Debug)]
pub struct RedDb {
    store: Store,
    query: Query,
    db_storage: Storage,
    opt_storage: Storage,
}

impl RedDb {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut buf = Vec::new();
        let db_storage = Storage::new(path)?;
        let opt_storage = Storage::new(".db.aof")?;
        db_storage.read_content(&mut buf);
        println!("[RedDb] Ok");

        Ok(Self {
            query: Query::new()?,
            store: Store::new(buf.lines())?,
            db_storage: db_storage,
            opt_storage: opt_storage,
        })
    }

    pub fn update(&mut self, query: Value, new_value: Value) -> Result<usize> {
        let mut store = self.store.write_store()?;
        let documents = self.query.update(&mut store, query, new_value)?;
        self.log_operation(&documents)?;
        Ok(documents.len())
    }

    pub fn delete(&mut self, query: Value) -> Result<usize> {
        let mut store = self.store.write_store()?;
        let documents = self.query.delete(&mut store, query)?;
        self.log_operation(&documents)?;
        Ok(documents.len())
    }

    pub fn flush_store(&self) -> Result<()> {
        self.store.flush_store()?;
        Ok(())
    }

    pub fn log_operation(&self, documents: &Vec<(&Uuid, &mut Document)>) -> Result<()> {
        let mut opt_storage = self.opt_storage.file.lock()?;
        let operation_log = self.store.format_operation(documents);
        opt_storage.seek(SeekFrom::End(0))?;
        opt_storage.write_all(&operation_log)?;
        opt_storage.sync_all()?;
        Ok(())
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
