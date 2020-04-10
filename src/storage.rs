use std::fs::{remove_file, File, OpenOptions};
use std::io::{Error, Read};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::result;
use std::sync::Mutex;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Storage {
    pub file_path: String,
    pub db_file: Mutex<File>,
}

impl Storage {
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {
            file_path: path.to_owned(),
            db_file: Mutex::new(
                OpenOptions::new()
                    .read(true)
                    .append(true)
                    .create(true)
                    .open(Path::new(path))?,
            ),
        })
    }

    pub fn read_content(&self, mut buf: &mut Vec<u8>) -> usize {
        self.db_file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap()
    }

    pub fn rebuild_storage<'a>(&'a self, data: &Vec<u8>) -> bool {
        let tmp = ".tmp";
        //FIXME
        if self.storage_exists() {
            println!("Rebuild");
            self.replace_data(tmp, data);
            self.replace_data(&self.file_path, data);
            remove_file(tmp).unwrap();
        }
        true
    }

    pub fn storage_exists<'a>(&'a self) -> bool {
        Path::new(&self.file_path).exists()
    }

    pub fn replace_data<'a, P: AsRef<Path>>(&'a self, path: P, data: &Vec<u8>) -> bool {
        let mut storage = File::create(path).unwrap();
        storage.set_len(10).unwrap();
        storage.seek(SeekFrom::Start(0)).unwrap();
        storage.write_all(&data).unwrap();
        storage.sync_all().unwrap();
        true
    }

    pub fn append_data<'a>(&'a self, data: &Vec<u8>) -> bool {
        let mut storage = self.db_file.lock().unwrap();
        storage.seek(SeekFrom::End(0)).unwrap();
        storage.write_all(&data).unwrap();
        storage.sync_all().unwrap();
        true
    }
}
