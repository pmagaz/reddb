use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::fs::{remove_file, File, OpenOptions};
use std::io::{Error, Read};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;

use super::Storage;
use crate::record::Record;
use crate::serializer::{Serializer, Serializers};
use crate::store::{Result, WriteOperation, WriteOperations};

#[derive(Debug)]
pub struct FileStorage<SE> {
    pub file_path: String,
    pub serializer: SE,
    pub db_file: Mutex<File>,
}

impl<SE> Storage for FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug,
{
    fn new<T>(path: &str) -> Result<Self> {
        Ok(Self {
            serializer: SE::default(),
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
    fn save<T>(&self, docs: WriteOperations<T>) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug,
    {
        let serialized: Vec<u8> = docs
            .into_iter()
            .map(|(id, value, operation)| Record::new(id, value, operation))
            .flat_map(|record| self.serializer.serialize(&record))
            .collect();
        self.append_data(&serialized);
        Ok(())
    }
    fn save_one<T>(&self, doc: WriteOperation<T>) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug,
    {
        let record = Record::new(doc.0, doc.1, doc.2);
        let serialized = self.serializer.serialize(&record);
        self.append_data(&serialized);
        Ok(())
    }
}

impl<SE> FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug,
{
    pub fn new2(path: &str) -> Result<Self> {
        Ok(Self {
            serializer: SE::default(),
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

    fn read_content(&self, mut buf: &mut Vec<u8>) -> usize {
        self.db_file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap()
    }

    fn rebuild_storage<'a>(&'a self, data: &Vec<u8>) -> bool {
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

    fn storage_exists<'a>(&'a self) -> bool {
        Path::new(&self.file_path).exists()
    }

    fn replace_data<'a, P: AsRef<Path>>(&'a self, path: P, data: &Vec<u8>) -> bool {
        let mut storage = File::create(path).unwrap();
        storage.set_len(10).unwrap();
        storage.seek(SeekFrom::Start(0)).unwrap();
        storage.write_all(&data).unwrap();
        storage.sync_all().unwrap();
        true
    }

    fn append_data<'a>(&'a self, data: &Vec<u8>) -> bool {
        let mut storage = self.db_file.lock().unwrap();
        storage.seek(SeekFrom::End(0)).unwrap();
        storage.write_all(&data).unwrap();
        storage.sync_all().unwrap();
        true
    }
}
