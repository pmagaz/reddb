use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::result;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use uuid::Uuid;

mod error;
mod handler;
mod json;
mod status;

use handler::Handler;
use status::Status;
pub type Result<T> = result::Result<T, error::DStoreError>;
pub type DStoreHashMap = HashMap<Uuid, Document>;

#[derive(Debug)]
pub struct DStore {
    //pub struct DStore<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    handler: Handler,
    store: RwLock<DStoreHashMap>,
}

//TODO REMOVE DUPLICATE HASHMAP K / ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    data: Value,
    #[serde(skip_serializing, skip_deserializing)]
    status: Status,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonDocument {
    _id: Value,
    data: Value,
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
        Ok(Self {
            handler: handler,
            store: RwLock::new(map),
        })
    }

    //TODO should return inserted value
    pub fn insert(&mut self, value: String) -> Value {
        let mut map = self.store.write().unwrap();
        let json_data: Value = serde_json::from_str(&value).unwrap();
        let doc = Document {
            data: json_data,
            status: Status::NotSaved,
        };
        let _id = Uuid::new_v4();
        let json_doc = self.to_jsondoc(&_id, &doc);
        map.insert(_id, doc);
        let leches = map.get(&_id);
        println!("LECHES{:?}", leches);

        json_doc
    }

    pub fn to_jsondoc(&self, _id: &Uuid, doc: &Document) -> Value {
        let mut json_value: Value = serde_json::to_value(doc).unwrap();
        json_value["_id"] = Value::String(_id.to_string());
        json_value.clone()
    }

    pub fn to_jsonresult(&self, _id: &Uuid, doc: &Document) -> Value {
        let mut json_value: Value = serde_json::to_value(doc).unwrap();
        json_value["data"]["_id"] = Value::String(_id.to_string());
        json_value["data"].clone()
    }

    pub fn find_by_id(&self, id: &Value) -> Value {
        let map = self.store.read().unwrap();
        let id = id.as_str().unwrap();
        let _id = Uuid::parse_str(id).unwrap();
        let doc = map.get(&_id).unwrap();
        let result = self.to_jsonresult(&_id, &doc);
        result
    }

    pub fn get(&self) {
        let map = self.store.read().unwrap();
        println!("STORE DATA{:?}", map);
    }

    pub fn jsondocs_tosave<'a>(&self, map: &'a RwLockReadGuard<DStoreHashMap>) -> Vec<Value> {
        let serialized_doc: Vec<Value> = map
            .iter()
            .filter(|(_k, v)| v.status == Status::NotSaved)
            .map(|(_id, doc)| self.to_jsondoc(&_id, &doc))
            .collect();
        serialized_doc
    }

    pub fn persist(&mut self) -> Result<()> {
        let map = self.store.read()?;
        let mut file = self.handler.file.lock()?;
        let json_docs = self.jsondocs_tosave(&map);
        let buf = json::serialize(&json_docs)?;
        //file.set_len(0);
        file.seek(SeekFrom::End(0))?;
        file.write_all(&buf)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
}
