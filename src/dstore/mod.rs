use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{BufRead, BufReader};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::result;
use std::sync::{RwLock, RwLockWriteGuard};

mod error;
mod handler;
//mod json;
mod ser;
use handler::Handler;
pub type Result<T> = result::Result<T, error::DStoreError>;

#[derive(Debug)]
pub struct DStore {
    //pub struct DStore<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    handler: Handler,
    store: RwLock<HashMap<String, Document>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    //pub struct Document<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    id: String,
    createdAt: String,
    data: Value,
}

impl DStore {
    //impl<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> DStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore> {
        let mut buf = Vec::new();
        let handler = Handler::new(path)?;
        //TODO IMPROVE READ LINE BY LINE
        handler
            .file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();

        let mut map: HashMap<String, Document> = HashMap::new();

        let lines = &mut buf.lines();
        for (_index, line) in lines.enumerate() {
            let content = &line.unwrap();
            let doc: Document = serde_json::from_str(content)?;
            let key = doc.id.clone();
            map.insert(key, doc);
        }
        Ok(Self {
            handler: handler,
            store: RwLock::new(map),
        })
    }

    pub fn get(&self) -> Result<()> {
        let data = self.store.read()?;
        println!("aaaaaaa{:?}", data.len());
        Ok(())
    }

    //TODO RETURN RESULT
    //TODO IMPLEMENT ONLY PERSISTS CHANGES
    pub fn put(&mut self, value: String) -> &mut DStore {
        {
            let mut lock = self.store.write().unwrap();
            let doc: Document = serde_json::from_str(&value).unwrap();
            lock.insert("value".to_string(), doc);
        }
        self
    }

    pub fn persist(&mut self) -> Result<()> {
        let map = self.store.read()?;
        let mut file = self.handler.file.lock()?;
        let buf = ser::serialize(&*map).unwrap();
        println!("aaaaaaa{}", map.len());

        file.set_len(0);
        file.seek(SeekFrom::Start(0));
        file.write_all(&buf);
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
}
