use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Error;
use std::io::Read;
use std::io::{BufRead, Seek, SeekFrom, Write};
use std::path::Path;
use std::result;
use std::sync::Mutex;
pub type Result<T> = result::Result<T, Error>;

//FIXME READ
#[derive(Debug)]
pub struct Storage {
    pub file: Mutex<File>,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let storage = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;

        println!("[RedDb] Seting up persistence");

        Ok(Self {
            file: Mutex::new(storage),
        })
    }

    //TODO read line by line
    pub fn read_content(&self, mut buf: &mut Vec<u8>) -> usize {
        println!("[RedDb] Reading database content");
        let content = self.file.try_lock().unwrap().read_to_end(&mut buf).unwrap();
        content
    }

    // pub fn log<T>(&self, documents: &Vec<(T)>) -> Vec<u8> {
    //     let formated_docs: Vec<u8> = documents
    //         .iter()
    //         //.filter(|(_id, doc)| doc.status != Status::Saved)
    //         .map(|(_id, doc)| json::to_jsonlog(&_id, &doc).unwrap())
    //         .flat_map(|doc| {
    //             let mut doc_vector = json::serialize(&doc).unwrap();
    //             doc_vector.extend("\n".as_bytes());
    //             doc_vector
    //         })
    //         .collect();
    //     formated_docs
    // }

    pub fn log<'a>(&'a self, doc: &Vec<u8>) -> bool {
        let mut storage = self.file.lock().unwrap();
        storage.seek(SeekFrom::End(0)).unwrap();
        storage.write_all(&doc).unwrap();
        storage.sync_all().unwrap();
        true
    }
}
