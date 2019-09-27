use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::result;
use std::sync::RwLockReadGuard;
use uuid::Uuid;

mod error;
mod handler;
mod json;
mod status;
mod store;

use handler::Handler;
use status::Status;
use store::{DStoreHashMap, Document, JsonDocument, Store};
pub type Result<T> = result::Result<T, error::DStoreError>;
//pub type DStoreHashMap = HashMap<Uuid, Document>;

#[derive(Debug)]
pub struct DStore {
    //pub struct DStore<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    handler: Handler,
    store: Store,
}

impl DStore {
    //impl<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> DStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore> {
        let mut buf = Vec::new();
        let handler = Handler::new(path)?;
        //TODO IMPROVE READ LINE BY LINE (split into a new function)
        handler
            .file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();

        let mut map: DStoreHashMap = HashMap::new();
        println!("[DStore] Parsing data into memory");

        for (_index, line) in buf.lines().enumerate() {
            let content = &line.unwrap();
            let json_doc: JsonDocument = serde_json::from_str(content)?;
            let _id = match &json_doc._id.as_str() {
                Some(_id) => Uuid::parse_str(_id).unwrap(),
                None => panic!("ERR: Wrong Uuid format!"),
            };
            let doc = Document {
                data: json_doc.data,
                status: Status::Saved,
            };
            map.insert(_id, doc);
        }
        println!("[DStore] Up & running");

        Ok(Self {
            handler: handler,
            store: Store::new(map)?,
        })
    }

    pub fn insert(&mut self, data: String) -> Value {
        self.store.insert(data).unwrap()
    }

    pub fn find_by_id(&self, id: &Value) -> Value {
        self.store.find_by_id(id).unwrap()
    }

    pub fn find(&self, data: String) -> Value {
        self.store.find(data).unwrap()
    }

    pub fn get(&self) {
        self.store.get().unwrap()
    }

    pub fn jsondocs_tosave<'a>(&self, store: &'a RwLockReadGuard<DStoreHashMap>) -> Vec<Value> {
        let serialized_doc: Vec<Value> = store
            .iter()
            .filter(|(_k, v)| v.status == Status::NotSaved)
            .map(|(_id, doc)| json::to_jsondoc(&_id, &doc).unwrap())
            .collect();
        serialized_doc
    }

    pub fn persist(&mut self) -> Result<()> {
        let store = self.store.data.read()?;
        let mut file = self.handler.file.lock()?;
        let json_docs = self.jsondocs_tosave(&store);
        let buf = json::serialize(&json_docs)?;
        //file.set_len(0);
        file.seek(SeekFrom::End(0))?;
        file.write_all(&buf)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
}
