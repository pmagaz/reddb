use std::fs::{File, OpenOptions};
use std::io::Error;
use std::io::Read;
use std::path::Path;
use std::result;
use std::sync::Mutex;

pub type Result<T> = result::Result<T, Error>;

//FIXME READ
#[derive(Debug)]
pub struct Handler {
    pub file: Mutex<File>,
}

impl Handler {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let handler = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;

        println!("[DStore] Seting up persistence");

        Ok(Self {
            file: Mutex::new(handler),
        })
    }

    //TODO read line by line
    pub fn read_content(&self, mut buf: &mut Vec<u8>) -> usize {
        println!("[DStore] Reading database content");
        let content = self.file.try_lock().unwrap().read_to_end(&mut buf).unwrap();
        content
    }
}
