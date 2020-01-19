use super::error;
use super::json;
use super::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::Lines;
use std::result;
use std::sync::RwLock;
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

    pub fn find(&self, query: String) -> Result<Value> {
        let query_object: Value = serde_json::from_str(&query)?;
        let result = match query_object.get("_id") {
            Some(_id) => self.find_by_id(&_id)?,
            None => self.find_by_data(&query_object)?,
        };
        Ok(result)
    }

    pub fn find_by_id(&self, id: &Value) -> Result<Value> {
        let store = self.store.read()?;
        let id = id.as_str().unwrap();
        let _id = Uuid::parse_str(id)?;
        let doc = store.get(&_id).unwrap();
        let result = json::to_jsonresult(&_id, &doc)?;
        Ok(result)
    }

    pub fn find_by_data(&self, query_object: &Value) -> Result<Value> {
        let store = self.store.read()?;
        let mut found = Vec::new();
        for (key, doc) in store.iter() {
            for (prop, value) in query_object.as_object().unwrap().iter() {
                match &doc.data.get(prop) {
                    Some(x) => {
                        if x == &value {
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

    pub fn insert(&mut self, value: String) -> Result<Value> {
        let mut store = self.store.write()?;
        let json_data: Value = serde_json::from_str(&value)?;
        let doc = Document {
            data: json_data,
            status: Status::NotSaved,
        };
        let _id = Uuid::new_v4();
        let json_doc = json::to_jsondoc(&_id, &doc)?;
        store.insert(_id, doc);
        Ok(json_doc)
    }

    pub fn delete(&self, query: String) -> Result<Value> {
        let query_object: Value = serde_json::from_str(&query)?;
        let result = match query_object.get("_id") {
            Some(_id) => self.find_by_id(&_id)?,
            None => self.find_by_data(&query_object)?,
        };
        Ok(result)
    }

    pub fn get(&self) -> Result<()> {
        let data = self.store.read()?;
        println!("STORE DATA{:?}", &data);
        Ok(())
    }

    pub fn format_jsondocs<'a>(&self) -> Vec<u8> {
        let data = self.store.read().unwrap();
        let formated_docs: Vec<u8> = data
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
