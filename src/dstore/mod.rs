use serde::{Deserialize, Serialize};
use serde_json::json;
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
    store: RwLock<HashMap<Uuid, Document>>,
}

//TODO REMOVE DUPLICATE HASHMAP K / ID
#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    #[serde(skip_serializing, skip_deserializing)]
    uuid: Uuid,
    #[serde(skip_serializing, skip_deserializing)]
    data: Value,
    #[serde(skip_serializing, skip_deserializing)]
    status: Status,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Doc2 {
    uuid: Uuid,
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
            let uuid = Uuid::new_v4();
            let data: Value = serde_json::from_str(content)?;
            // println!("DATA{:?}", data);
            let doc = Document {
                uuid: Uuid::new_v4(),
                data: data,
                status: Status::NotSaved,
            };
            map.insert(uuid, doc);
        }
        Ok(Self {
            handler: handler,
            store: RwLock::new(map),
        })
    }

    pub fn create_doc(&self, data: Value) -> Document {
        Document {
            uuid: Uuid::new_v4(),
            data: data,
            status: Status::NotSaved,
        }
    }

    //TODO should return inserted value
    pub fn insert(&mut self, value: String) -> &mut DStore {
        {
            let mut map = self.store.write().unwrap();
            let data: Value = serde_json::from_str(&value).unwrap();
            println!("DATA{:?}", data);
            let doc = self.create_doc(data);
            map.insert(doc.uuid.clone(), doc);
        }
        self
    }

    pub fn get(&self) {
        let map = self.store.read().unwrap();
        println!("STORE LEN{:?}", map);
    }

    pub fn data_to_persist<'a>(&self, map: &'a RwLockReadGuard<DStoreHashMap>) -> Vec<Value> {
        let new_map: Vec<Value> = map
            .iter()
            .filter(|(_, doc)| doc.status == Status::NotSaved)
            .map(|(_, doc)| {
                let mut value = json!({
                    "uuid": doc.uuid,
                });
                //let mut object = Value::new_object();
                for (k, v) in doc.data.as_object().unwrap().iter() {
                    println!("NEW_MAP{:?} {:?}", k, v);
                    //value.put(k, v);
                    //value[k] = v;
                    //k
                }
                //println!("NEW_MAP{:?}", value);
                value
                // Value {
                //     uuid: doc.uuid,
                //     data: doc.data.clone(),
                // }
                //doc
            })
            .collect();
        new_map
    }

    //TODO split new_map into a new function
    //TODO should add \n after every hashmap
    pub fn persist(&mut self) -> Result<()> {
        let map = self.store.read()?;
        let mut file = self.handler.file.lock()?;
        let data_to_persist = self.data_to_persist(&map);
        let buf = json::serialize(&data_to_persist)?;

        println!("STORE LEN{:?}", buf);
        //file.set_len(0);
        file.seek(SeekFrom::End(0))?;
        // file.write_all(&buf)?;
        // file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
}
