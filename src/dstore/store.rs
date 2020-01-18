use super::error;
use super::json;
use super::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub data: RwLock<DStoreHashMap>,
}

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
            data: RwLock::new(map_store),
        })
    }

    pub fn insert(&mut self, value: String) -> Result<Value> {
        let mut store = self.data.write()?;
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

    pub fn find_by_id(&self, id: &Value) -> Result<Value> {
        //TODO fix error
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
                            found.push(json::to_jsonresult(&key, &doc)?)
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
