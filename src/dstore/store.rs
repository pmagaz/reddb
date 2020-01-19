use super::error;
use super::json;
use super::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Lines;
use std::result;
use std::sync::{RwLock, RwLockReadGuard};
use uuid::Uuid;

pub type Result<T> = result::Result<T, error::DStoreError>;
pub type DStoreHashMap = HashMap<Uuid, Document>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub data: Value,
    #[serde(skip_serializing, skip_deserializing)]
    pub status: Status,
}

#[derive(Debug)]
pub struct Store {
    pub store: RwLock<DStoreHashMap>,
}
// FIXME unwraps
impl Store {
    pub fn new(buf: Lines<&[u8]>) -> Result<Store> {
        println!("[DStore] Parsing database into memory");
        let mut map_store: DStoreHashMap = HashMap::new();
        for (_index, line) in buf.enumerate() {
            let content = &line?;
            let json_doc = json::from_str(&content)?;
            let _id = match &json_doc._id.as_str() {
                Some(_id) => Uuid::parse_str(_id)?,
                None => panic!("ERR: Wrong Uuid format!"),
            };
            let doc = Document {
                data: json_doc.data,
                status: Status::Saved,
            };
            map_store.insert(_id, doc);
        }

        Ok(Self {
            store: RwLock::new(map_store),
        })
    }

    pub fn read_store(&self) -> Result<RwLockReadGuard<DStoreHashMap>> {
        Ok(self.store.read()?)
    }

    pub fn find(&self, query: &Value) -> Result<Value> {
        let result = self.find_data(&query, false)?;
        Ok(result)
    }

    pub fn find_one(&self, query: &Value) -> Result<Value> {
        let result = match query.get("_id") {
            Some(_id) => self.find_by_id(&_id)?,
            None => self.find_data(&query, true)?,
        };
        Ok(result)
    }

    pub fn find_by_id(&self, id: &Value) -> Result<Value> {
        let store = self.read_store()?;
        let id = id.as_str().unwrap();
        let _id = Uuid::parse_str(id)?;
        let doc = store.get(&_id).unwrap();
        let result = json::to_jsonresult(&_id, &doc)?;
        Ok(result)
    }

    pub fn find_data(&self, query: &Value, find_one: bool) -> Result<Value> {
        let store = self.read_store()?;
        let mut docs_founded = Vec::new();
        let query_map = query.as_object().unwrap();
        for (key, doc) in store.iter() {
            let mut properties_match: Vec<i32> = Vec::new();
            let num_properties = query_map.len();
            if find_one && docs_founded.len() == 1 {
                break;
            }
            for (prop, value) in query_map.iter() {
                match &doc.data.get(prop) {
                    Some(item) => {
                        if item == &value {
                            properties_match.push(1);
                            if num_properties == properties_match.len() {
                                docs_founded.push(json::to_jsonresult(&key, &doc)?)
                            }
                        }
                    }
                    None => (),
                };
            }
        }
        let result = Value::Array(docs_founded);
        Ok(result)
    }

    //TODO update multiple fields
    pub fn update(&self, query: Value, newValue: Value) -> Result<Value> {
        let store = self.read_store()?;
        let mut found = Vec::new();
        for (key, doc) in store.iter() {
            for (prop, value) in query.as_object().unwrap().iter() {
                match doc.data.get(prop) {
                    Some(item) => {
                        if item == value {
                            found.push(json::to_jsonresult(&key, &doc)?)
                        }
                    }
                    None => (),
                };
            }
        }
        let result = Value::Array(found);
        Ok(result)
    }

    pub fn insert(&mut self, query: Value) -> Result<Value> {
        let mut store = self.store.write()?;
        let doc = Document {
            data: query,
            status: Status::NotSaved,
        };
        let _id = Uuid::new_v4();
        let json_doc = json::to_jsondoc(&_id, &doc)?;
        store.insert(_id, doc);
        Ok(json_doc)
    }

    pub fn delete(&self, query: Value) -> Result<Value> {
        let result = match query.get("_id") {
            Some(_id) => self.find_by_id(&_id)?,
            None => self.find_data(&query, true)?,
        };
        Ok(result)
    }

    pub fn format_jsondocs<'a>(&self) -> Vec<u8> {
        let store = self.read_store().unwrap();
        let formated_docs: Vec<u8> = store
            .iter()
            .filter(|(_k, v)| v.status == Status::NotSaved)
            .map(|(_id, doc)| json::to_jsondoc(&_id, &doc).unwrap())
            .flat_map(|doc| {
                let mut doc_vector = json::serialize(&doc).unwrap();
                doc_vector.extend("\n".as_bytes());
                doc_vector
            })
            .collect();
        formated_docs
    }
}
