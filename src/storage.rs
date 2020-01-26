use std::fs::{File, OpenOptions};
use std::io::Error;
use std::io::Read;
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
}
