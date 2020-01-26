use super::error;
use super::json;
use super::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Lines;
use std::result;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

pub type Result<T> = result::Result<T, error::RedDbError>;
pub type RedDbHashMap = HashMap<Uuid, Document>;
pub type WriteGuard<'a, T> = RwLockWriteGuard<'a, T>;
pub type ReadGuard<'a, T> = RwLockReadGuard<'a, T>;

#[derive(Debug)]
pub struct Store {
    pub store: RwLock<RedDbHashMap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub data: Value,
    #[serde(skip_deserializing)]
    pub status: Status,
}

// FIXME unwraps
impl Store {
    pub fn new(buf: Lines<&[u8]>) -> Result<Self> {
        println!("[RedDb] Parsing database into memory");
        let mut map_store: RedDbHashMap = HashMap::new();
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

    pub fn read_store(&self) -> Result<ReadGuard<RedDbHashMap>> {
        Ok(self.store.read()?)
    }

    pub fn write_store(&self) -> Result<WriteGuard<RedDbHashMap>> {
        Ok(self.store.write()?)
    }

    pub fn flush_store(&self) -> Result<()> {
        let store = self.read_store().unwrap();
        for (_key, doc) in store.iter() {
            println!("STORE RECORD {:?}", doc);
        }
        Ok(())
    }

    pub fn format_jsondocs(&self) -> Vec<u8> {
        let store = self.read_store().unwrap();
        println!("STORE DATA{:?}", &store);
        let formated_docs: Vec<u8> = store
            .iter()
            .filter(|(_k, v)| v.status == Status::NotSaved)
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
