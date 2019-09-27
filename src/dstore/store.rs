use std::collections::HashMap;
use std::io::Error;
use std::path::Path;
use std::result;
use std::sync::{Mutex, RwLock, RwLockWriteGuard};
use uuid::Uuid;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Document {
    data: String,
}

#[derive(Debug)]
pub struct Store {
    store: RwLock<HashMap<Uuid, Document>>,
}

impl Store {
    pub fn new(data: HashMap<Uuid, Document>) -> Result<Store> {
        Ok(Store {
            store: RwLock::new(data),
        })
    }
}
