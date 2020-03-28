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

        Ok(Self {
            file: Mutex::new(storage),
        })
    }

    //TODO read line by line
    pub fn read_content(&self, mut buf: &mut Vec<u8>) -> usize {
        let content = self.file.try_lock().unwrap().read_to_end(&mut buf).unwrap();
        content
    }

    pub fn write<'a>(&'a self, data: &Vec<u8>) -> bool {
        let mut storage = self.file.lock().unwrap();
        storage.seek(SeekFrom::End(0)).unwrap();
        storage.write_all(&data).unwrap();
        storage.sync_all().unwrap();
        true
    }
}
