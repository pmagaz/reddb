use super::error;
use super::json;
use super::status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::result;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use uuid::Uuid;

pub type Result<T> = result::Result<T, error::DStoreError>;
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

    pub fn insert(&mut self, value: String) -> Result<Value> {
        let mut store = self.data.write()?;
        let json_data: Value = serde_json::from_str(&value)?;
        let doc = Document {
            data: json_data,
            status: status::Status::NotSaved,
        };
        let _id = Uuid::new_v4();
        let json_doc = json::to_jsondoc(&_id, &doc)?;
        store.insert(_id, doc);
        Ok(json_doc)
    }

    pub fn find_by_id(&self, id: &Value) -> Result<Value> {
        let store = self.data.read()?;
        let id = id.as_str().unwrap();
        let _id = Uuid::parse_str(id)?;
        let doc = store.get(&_id).unwrap();
        let result = json::to_jsonresult(&_id, &doc)?;
        Ok(result)
    }

    pub fn find(&self, data: String) -> Result<Value> {
        let store = self.data.read()?;
        let json_data: Value = serde_json::from_str(&data)?;
        let mut found = Vec::new();
        for (key, doc) in store.iter() {
            for (prop, value) in json_data.as_object().unwrap().iter() {
                match &doc.data.get(prop) {
                    Some(x) => {
                        if x == &value {
                            found.push(json::to_jsonresult(&key, &doc).unwrap())
                        }
                    }
                    None => (),
                };
            }
        }
        let docs = Value::Array(found);
        println!("FIND RESULT {:?}", docs);
        Ok(docs)
    }

    pub fn get(&self) -> Result<()> {
        let data = self.data.read()?;
        println!("STORE DATA{:?}", &data);
        Ok(())
    }

    pub fn format_jsondocs<'a>(&self) -> Vec<u8> {
        let data = self.data.read().unwrap();

        let formated_docs: Vec<u8> = data
            .iter()
            .filter(|(_k, v)| v.status == status::Status::NotSaved)
            .map(|(_id, doc)| json::to_jsondoc(&_id, &doc).unwrap())
            .flat_map(|doc| {
                let mut doc_vector = serde_json::to_vec(&doc).unwrap();
                doc_vector.extend("\n".as_bytes());
                doc_vector
            })
            .collect();
        formated_docs
    }
}
