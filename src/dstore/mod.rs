use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::result;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

mod error;
mod handler;
//mod json;
mod ser;
use handler::Handler;
pub type Result<T> = result::Result<T, error::DStoreError>;
pub type DStoreHashMap = HashMap<String, Document>;
pub type DStoreSaveHashMap = HashMap<String, Status>;

#[derive(Debug)]
pub enum Status {
    Saved,
    NotSaved,
}

#[derive(Debug)]
pub struct DStore {
    //pub struct DStore<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    handler: Handler,
    toSave: ToSaveData,
    store: RwLock<HashMap<String, Document>>,
}

#[derive(Debug)]
pub struct ToSaveData {
    data: HashMap<String, Status>,
    //updater: Updater
}

impl ToSaveData {
    pub fn addToQue(&mut self, key: String, value: Status) {
        self.data.insert(key, value);
    }
    pub fn changeValue(&mut self, key: &str, data: &mut DStoreSaveHashMap) {
        let value = data.get_mut(key);
    }
    pub fn getNewMap<'a>(
        &mut self,
        map: RwLockReadGuard<'a, DStoreHashMap>,
    ) -> RwLockReadGuard<'a, DStoreHashMap> {
        let mut data = &self.data;
        //let mapStore = self.store.read().unwrap();
        //CONTINUE HERE
        for (k, v) in &data {
            let value = map.get(k).unwrap();
            //self.changeValue(k, &mut data);
            data.get_mut(k);
            //let new_value = &data.get_mut(k);
            println!("K{:?}", k);
            println!("V{:?}", value);
        }
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    //pub struct Document<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> {
    id: String,
    createdAt: String,
    data: Value,
}

impl DStore {
    //impl<T: std::fmt::Debug + Eq + Hash + Serialize + DeserializeOwned> DStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<DStore> {
        let mut buf = Vec::new();
        let handler = Handler::new(path)?;
        //TODO IMPROVE READ LINE BY LINE
        handler
            .file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();

        let mut map: DStoreHashMap = HashMap::new();

        for (_index, line) in buf.lines().enumerate() {
            let content = &line.unwrap();
            let doc: Document = serde_json::from_str(content)?;
            let key = doc.id.clone();
            map.insert(key, doc);
        }
        Ok(Self {
            handler: handler,
            toSave: ToSaveData {
                data: HashMap::new(),
            },
            store: RwLock::new(map),
        })
    }

    pub fn get(&self) -> Result<()> {
        let data = self.store.read()?;
        //println!("aaaaaaa{:?}", data.len());
        Ok(())
    }

    //TODO RETURN RESULT
    //TODO IMPLEMENT ONLY PERSISTS CHANGES
    pub fn insert(&mut self, value: String) -> &mut DStore {
        {
            let mut map = self.store.write().unwrap();
            let doc: Document = serde_json::from_str(&value).unwrap();
            let key = doc.id.clone();
            map.insert(key.clone(), doc);
            self.toSave.addToQue(key, Status::NotSaved);
        }
        self
    }

    // pub fn getNewMap<'a>(
    //     &mut self,
    //     map: DStoreSaveHashMap,
    // ) -> RwLockReadGuard<'a, DStoreHashMap> {
    //     //let data = &mut self.data;
    //     //let mapStore = self.store.read().unwrap();
    //     let map2 = self.store.read().unwrap();

    //     for (k, v) in map {
    //         let value = map.get(&k);
    //         //self.changeValue(k, &data);//data.get_mut(k);
    //         //let new_value = &data.get_mut(k);
    //         println!("K{:?}", k);
    //         println!("V{:?}", value);
    //     }
    //     map2
    // }

    pub fn persist(&mut self) -> Result<()> {
        let map = self.store.read()?;
        let mut file = self.handler.file.lock()?;
        let map2 = self.toSave.getNewMap(map);
        let buf = ser::serialize(&*map2).unwrap();
        //let data = &mut self.toSave;
        // println!("aaaaaaa{:?}", map.len());
        // for (k, v) in data {
        //     println!("{:?}", k);

        //     //  let doc = match map.get(&k) {
        //     //     Some(val) => val,
        //     //     None => "",
        //     //  };
        //     let value = map.get(k);
        //     //let change = &*data.get_mut(k);

        //     println!("{:?}", value);
        // }

        //let map : <HashMap<String, Document> = &self.save.map(

        // file.set_len(0);
        // file.seek(SeekFrom::Start(0));
        // file.write_all(&buf);
        // file.write_all(b"\n")?;
        // file.sync_all()?;
        Ok(())
    }
}
