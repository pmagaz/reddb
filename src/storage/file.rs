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
use crate::kv::KeyValue;
use crate::serializer::{Serializer, Serializers};
use crate::{ByteString, StoreHM};

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

    fn save<T>(&self, data: Vec<KeyValue<T>>) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug,
    {
        let serialized: Vec<u8> = data
            .into_iter()
            //.map(|(id, value)| KeyValue::new(id, value))
            .flat_map(|record| self.serializer.serialize(&record).unwrap())
            .collect();
        self.append_data(&serialized)
            .context(RdStoreErrorKind::AppendData)?;
        Ok(())
    }
    fn save_one<T>(&self, doc: KeyValue<T>) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug,
    {
        //let record = Record::new(doc.0, doc.1);
        let serialized = self.serializer.serialize(&doc).unwrap();
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
            let record: KeyValue<T> = self
                .serializer
                .deserialize(byte_str)
                .context(RdStoreErrorKind::DataCorruption)
                .unwrap();

            let key = record.key;
            let value = record.value;
            let serialized = self.serializer.serialize(&value).unwrap();
            map.insert(key, Mutex::new(serialized));
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
                KeyValue::new(*id, data)
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
