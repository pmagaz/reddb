use core::fmt::Debug;
use failure::{Error, ResultExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{remove_file, File, OpenOptions};
use std::io::BufRead;
use std::io::Read;

use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;

use super::Storage;
use crate::error::{RdStoreErrorKind, Result};
use crate::record::Record;
use crate::serializer::{Serializer, Serializers};
use crate::store::{ByteString, StoreHM, WriteOperation, WriteOperations};
use crate::Operation;

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
    fn new<T>() -> Result<Self> {
        let serializer = SE::default();
        let file_name = match serializer.format() {
            Serializers::Json(st) => st,
            Serializers::Yaml(st) => st,
            Serializers::Ron(st) => st,
        };
        Ok(Self {
            serializer: SE::default(),
            file_path: file_name.to_owned(),
            db_file: Mutex::new(
                OpenOptions::new()
                    .read(true)
                    .append(true)
                    .create(true)
                    .open(Path::new(file_name))
                    .map_err(|_| RdStoreErrorKind::StorageOpen)?,
            ),
        })
    }

    fn save<T>(&self, data: WriteOperations<T>) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug,
    {
        let serialized: Vec<u8> = data
            .into_iter()
            .map(|(id, value, operation)| Record::new(id, value, operation))
            .flat_map(|record| self.serializer.serialize(&record).unwrap())
            .collect();
        self.append_data(&serialized)
            .context(RdStoreErrorKind::AppendData)?;
        Ok(())
    }
    fn save_one<T>(&self, doc: WriteOperation<T>) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug,
    {
        let record = Record::new(doc.0, doc.1, doc.2);
        let serialized = self.serializer.serialize(&record).unwrap();
        self.append_data(&serialized)
            .context(RdStoreErrorKind::AppendData)?;
        Ok(())
    }
    fn load_content<T>(&self) -> Result<StoreHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let mut map: StoreHM = HashMap::new();
        let mut buf = Vec::new();
        self.read_content(&mut buf)
            .context(RdStoreErrorKind::ContentLoad)?;

        for (_index, content) in buf.lines().enumerate() {
            let line = content.unwrap();
            let byte_str = &line.into_bytes();
            let record: Record<T> = self
                .serializer
                .deserialize(byte_str)
                .context(RdStoreErrorKind::DataCorruption)
                .unwrap();

            let id = record._id;
            let data = record.data;
            match record.operation {
                Operation::Insert => {
                    let serialized = self.serializer.serialize(&data).unwrap();
                    map.insert(id, Mutex::new(serialized));
                }
                Operation::Update => {
                    match map.get_mut(&id) {
                        Some(value) => {
                            let mut guard = value.lock().unwrap();
                            *guard = self
                                .serializer
                                .serialize(&data)
                                .context(RdStoreErrorKind::DataCorruption)
                                .unwrap();
                        }
                        None => {}
                    };
                }
                Operation::Delete => {}
            }
        }

        self.compact_storage::<T>(&map)
            .context(RdStoreErrorKind::Compact)?;
        Ok(map)
    }
}

impl<SE> FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug,
{
    fn read_content(&self, mut buf: &mut Vec<u8>) -> Result<usize> {
        let content = self
            .db_file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .context(RdStoreErrorKind::ReadContent)?;
        Ok(content)
    }

    pub fn compact_storage<T>(&self, data: &StoreHM) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let tmp = ".tmp";
        let data: ByteString = data
            .iter()
            .map(|(id, value)| (id, value.lock().unwrap()))
            .map(|(id, value)| {
                let data: T = self
                    .serializer
                    .deserialize(&*value)
                    .context(RdStoreErrorKind::DataCorruption)
                    .unwrap();
                Record::new(*id, data, Operation::default())
            })
            .flat_map(|record| {
                self.serializer
                    .serialize(&record)
                    .context(RdStoreErrorKind::DataCorruption)
                    .unwrap()
            })
            .collect();

        if self.storage_exists() {
            self.flush_data(tmp, &data)?;
            self.flush_data(&self.file_path, &data)?;
            remove_file(tmp).unwrap();
        }
        Ok(())
    }

    fn storage_exists<'a>(&'a self) -> bool {
        Path::new(&self.file_path).exists()
    }

    fn flush_data<'a, P: AsRef<Path>>(&'a self, path: P, data: &Vec<u8>) -> Result<()> {
        let mut storage = File::create(path).context(RdStoreErrorKind::DataCorruption)?;
        storage.set_len(0).context(RdStoreErrorKind::FlushData)?;
        storage
            .seek(SeekFrom::Start(0))
            .context(RdStoreErrorKind::FlushData)?;
        storage
            .write_all(&data)
            .context(RdStoreErrorKind::FlushData)?;
        storage.sync_all().context(RdStoreErrorKind::FlushData)?;
        Ok(())
    }

    fn append_data<'a>(&'a self, data: &Vec<u8>) -> Result<()> {
        let mut storage = self.db_file.lock().unwrap();
        storage
            .seek(SeekFrom::End(0))
            .context(RdStoreErrorKind::AppendData)?;
        storage
            .write_all(&data)
            .context(RdStoreErrorKind::AppendData)?;
        storage.sync_all().context(RdStoreErrorKind::AppendData)?;
        Ok(())
    }
}
