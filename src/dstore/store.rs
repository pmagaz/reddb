use super::error;
use super::json;
use super::status::Status;
use bincode;
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
    #[serde(skip_deserializing)]
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

    pub fn get_id<'a>(&self, query: &'a Value) -> Result<&'a str> {
        //Fixme
        let _id = match query.get("_id").unwrap().as_str() {
            Some(_id) => _id,
            None => "",
        };
        Ok(_id)
    }

    pub fn get_uuid(&self, query: &Value) -> Result<Uuid> {
        let _id = self.get_id(query)?;
        let uuid = Uuid::parse_str(_id)?;
        Ok(uuid)
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

    pub fn find_id(&self, query: &Value) -> Result<Value> {
        let store = self.read_store()?;
        let uuid = self.get_uuid(&query)?;
        let doc = store.get(&uuid).unwrap();
        let result = json::to_jsonresult(&uuid, &doc)?;
        Ok(result)
    }

    //TODO unify find, update, delete
    pub fn find(&self, query: &Value) -> Result<Value> {
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

    pub fn update(&self, query: Value, new_value: Value) -> Result<Vec<Value>> {
        let mut store = self.write_store()?;
        let mut docs_updated = Vec::new();
        let query_map = query.as_object().unwrap();
        for (key, doc) in store.iter_mut() {
            let mut properties_match: Vec<i32> = Vec::new();
            let num_properties = query_map.len();
            for (prop, value) in query_map.iter() {
                match doc.data.get(prop) {
                    Some(val) => {
                        if val == value {
                            properties_match.push(1);
                            *doc.data.get_mut(prop).unwrap() = json!(new_value[prop]);
                            if num_properties == properties_match.len() {
                                doc.status = Status::Updated;
                                //FIXME it has to be a reference
                                docs_updated.push(json::to_jsonresult(&key, &doc)?)
                            }
                        }
                    }
                    None => (),
                };
            }
        }
        let result = docs_updated;
        Ok(result)
    }

    pub fn get_tuple<'a>(&self, key: (&'a Uuid, &'a Document)) -> (&'a Uuid, &'a Document) {
        key
    }

    pub fn update_doc<'a>(&self, doc: &'a mut Document, status: Status) -> &'a mut Document {
        doc.status = status;
        doc
    }

    pub fn delete<'a>(
        &self,
        store: &'a mut RwLockWriteGuard<DStoreHashMap>,
        query: Value,
    ) -> Result<Vec<(&'a Uuid, &'a mut Document)>> {
        let query_map = query.as_object().unwrap();
        let docs_deleted: Vec<(&Uuid, &mut Document)> = store
            .iter_mut()
            .map(|(key, doc)| {
                let mut properties_match: Vec<i32> = Vec::new();
                let num_properties = query_map.len();
                for (prop, value) in query_map.iter() {
                    match doc.data.get(prop) {
                        Some(val) => {
                            if val == value {
                                properties_match.push(1);
                                if num_properties == properties_match.len() {
                                    self.update_doc(doc, Status::Deleted);
                                }
                            }
                        }
                        None => (),
                    };
                }
                (key, doc)
            })
            .collect();

        let result = docs_deleted;
        Ok(result)
    }

    pub fn get(&self) -> Result<()> {
        let store = self.read_store().unwrap();
        for (key, doc) in store.iter() {
            println!("STORE DATA{:?}", doc);
        }
        Ok(())
    }

    pub fn format_jsondocs(&self) -> Vec<u8> {
        let store = self.read_store().unwrap();
        println!("STORE DATA{:?}", &store);
        let formated_docs: Vec<u8> = store
            .iter()
            .filter(|(_k, v)| v.status == Status::NotSaved)
            // .map(|doc| json::to_jsondoc(&_id & doc).unwrap())
            .flat_map(|doc| {
                let mut doc_vector = json::serialize(&doc).unwrap();
                doc_vector.extend("\n".as_bytes());
                doc_vector
            })
            .collect();
        formated_docs
    }

    pub fn format_operation(&self, documents: &Vec<(&Uuid, &mut Document)>) -> Vec<u8> {
        let formated_docs: Vec<u8> = documents
            .iter()
            .filter(|(_id, doc)| doc.status != Status::Saved)
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
