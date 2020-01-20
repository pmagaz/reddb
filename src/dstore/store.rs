use super::error;
use super::json;
use super::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Lines;
use std::result;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
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

    pub fn write_store(&self) -> Result<RwLockWriteGuard<DStoreHashMap>> {
        Ok(self.store.write()?)
    }

    pub fn find(&self, query: &Value) -> Result<Value> {
        let result = match query.get("_id") {
            Some(_id) => self.find_id(&_id)?,
            None => self.find_data(&query)?,
        };
        Ok(result)
    }

    pub fn find_id(&self, id: &Value) -> Result<Value> {
        let store = self.read_store()?;
        let id = id.as_str().unwrap();
        let _id = Uuid::parse_str(id)?;
        let doc = store.get(&_id).unwrap();
        let result = json::to_jsonresult(&_id, &doc)?;
        Ok(result)
    }

    pub fn find_data(&self, query: &Value) -> Result<Value> {
        let store = self.read_store()?;
        let mut docs_founded = Vec::new();
        let query_map = query.as_object().unwrap();
        for (key, doc) in store.iter() {
            let mut properties_match: Vec<i32> = Vec::new();
            let num_properties = query_map.len();
            for (prop, value) in query_map.iter() {
                match &doc.data.get(prop) {
                    Some(val) => {
                        if val == &value {
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
    pub fn update(&self, query: Value, newValue: Value) -> Result<usize> {
        let mut store = self.write_store()?;
        let mut docs_founded = Vec::new();
        let query_map = query.as_object().unwrap();
        for (key, doc) in store.iter_mut() {
            let mut properties_match: Vec<i32> = Vec::new();
            let num_properties = query_map.len();
            for (prop, value) in query_map.iter() {
                match doc.data.get(prop) {
                    Some(val) => {
                        if val == value {
                            properties_match.push(1);
                            *doc.data.get_mut(prop).unwrap() = json!(newValue[prop]);
                            if num_properties == properties_match.len() {
                                docs_founded.push(json::to_jsonresult(&key, &doc)?)
                            }
                        }
                    }
                    None => (),
                };
            }
            doc.status = Status::Updated;
        }
        let result = docs_founded.len();
        Ok(result)
    }

    pub fn insert(&mut self, query: Value) -> Result<Value> {
        let mut store = self.write_store()?;
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
            Some(_id) => self.find_id(&_id)?,
            None => self.find_data(&query)?,
        };
        Ok(result)
    }

    pub fn get(&self) -> Result<()> {
        let store = self.read_store().unwrap();
        println!("STORE DATA{:?}", &store);
        Ok(())
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
