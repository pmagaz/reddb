use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Error;
use std::result;
use std::sync::RwLock;
use uuid::Uuid;

use super::json;
use super::status;

pub type Result<T> = result::Result<T, Error>;
pub type DStoreHashMap = HashMap<Uuid, Document>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub data: Value,
    #[serde(skip_serializing, skip_deserializing)]
    pub status: status::Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonDocument {
    pub _id: Value,
    pub data: Value,
}

#[derive(Debug)]
pub struct Store {
    pub data: RwLock<DStoreHashMap>,
}

impl Store {
    pub fn new(data: DStoreHashMap) -> Result<Store> {
        Ok(Store {
            data: RwLock::new(data),
        })
    }

    pub fn insert(&mut self, value: String) -> Value {
        let mut store = self.data.write().unwrap();
        let json_data: Value = serde_json::from_str(&value).unwrap();
        let doc = Document {
            data: json_data,
            status: status::Status::NotSaved,
        };
        let _id = Uuid::new_v4();
        let json_doc = json::to_jsondoc(&_id, &doc).unwrap();
        store.insert(_id, doc);
        json_doc
    }

    pub fn find_by_id(&self, id: &Value) -> Value {
        let store = self.data.read().unwrap();
        let id = id.as_str().unwrap();
        let _id = Uuid::parse_str(id).unwrap();
        let doc = store.get(&_id).unwrap();
        let result = json::to_jsonresult(&_id, &doc).unwrap();
        result
    }

    pub fn get(&self) {
        let store = self.data.read().unwrap();
        println!("STORE DATA{:?}", store);
    }
}
