use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Debug;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::result;
use std::sync::RwLock;

mod error;
mod handler;
mod json;
mod status;

use handler::Handler;
use status::Status;
pub type Result<T> = result::Result<T, error::DStoreError>;
pub type DStoreHashMap = HashMap<String, Document>;

#[derive(Debug)]
pub struct DStore {
    //pub struct DStore<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    handler: Handler,
    store: RwLock<HashMap<String, Document>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    id: String,
    created_at: String,
    data: Value,
    #[serde(skip_serializing, skip_deserializing)]
    status: Status,
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

        let mut map: DStoreHashMap = HashMap::new();

        for (_index, line) in buf.lines().enumerate() {
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
        Ok(())
    }

    //TODO should return inserted value
    pub fn insert(&mut self, value: String) -> &mut DStore {
        {
            let mut map = self.store.write().unwrap();
            let mut doc: Document = serde_json::from_str(&value).unwrap();
            let key = doc.id.clone();
            doc.status = Status::NotSaved;
            map.insert(key.clone(), doc);
        }
        self
    }

    pub fn persist(&mut self) -> Result<()> {
        let map = self.store.read()?;
        let mut file = self.handler.file.lock()?;
        let new_map: HashMap<&String, &Document> = map
            .iter()
            .filter(|(_k, v)| v.status == Status::NotSaved)
            .collect();
        let buf = json::serialize(&new_map).unwrap();
        //file.set_len(0);
        file.seek(SeekFrom::End(0));
        file.write_all(&buf)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
}
