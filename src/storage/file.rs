use core::fmt::Debug;
use failure::ResultExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::io::Read;

use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;

use super::Storage;
use crate::document::Document;
use crate::error::{RedDbErrorKind, Result};
use crate::serializer::{Serializer, Serializers};
use crate::RedDbHM;

#[derive(Debug)]
pub struct FileStorage<SE> {
    file_path: String,
    serializer: SE,
    db_file: Mutex<File>,
}

impl<SE> Storage for FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug,
{
    fn new(db_name: &str) -> Result<Self> {
        let serializer = SE::default();
        let extension = match serializer.format() {
            Serializers::Json(st) => st,
            Serializers::Yaml(st) => st,
            Serializers::Ron(st) => st,
        };

        let db_path = [db_name, extension].concat();

        Ok(Self {
            serializer: SE::default(),
            file_path: db_path.to_owned(),
            db_file: Mutex::new(
                OpenOptions::new()
                    .read(true)
                    .append(true)
                    .create(true)
                    .open(db_path)
                    .map_err(|_| RedDbErrorKind::StorageOpen)?,
            ),
        })
    }

    fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let mut map: RedDbHM = HashMap::new();
        let mut buf = Vec::new();
        self.read_data(&mut buf)
            .context(RedDbErrorKind::ContentLoad)?;

        for (_index, content) in buf.lines().enumerate() {
            let line = content.unwrap();
            let byte_str = &line.into_bytes();
            let document: Document<T> = self
                .serializer
                .deserialize(byte_str)
                .context(RedDbErrorKind::DataCorruption)
                .unwrap();

            let id = document.id;
            let data = document.data;
            let serialized = self.serializer.serialize(&data).unwrap();
            // map.insert(id, Mutex::new(serialized));
            map.entry(id).or_insert_with(|| Mutex::new(serialized));
        }

        // self.compact_data::<T>(&map)
        //     .context(RedDbErrorKind::Compact)?;
        //println!("{:?}leches", map.len());
        Ok(map)
    }

    fn persist<T>(&self, data: &[Document<T>]) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug,
    {
        let serialized: Vec<u8> = data
            .iter()
            .flat_map(|document| self.serializer.serialize(document).unwrap())
            .collect();

        self.append(&serialized)
            .context(RedDbErrorKind::AppendData)?;
        Ok(())
    }
}

impl<SE> FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug,
{
    fn read_data(&self, mut buf: &mut Vec<u8>) -> Result<usize> {
        let content = self
            .db_file
            .try_lock()
            .unwrap()
            .read_to_end(&mut buf)
            .context(RedDbErrorKind::ReadContent)?;

        Ok(content)
    }

    pub fn compact_data<T>(&self, data: &RedDbHM) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data: Vec<u8> = data
            .iter()
            .map(|(id, data)| (id, data.lock().unwrap()))
            .map(|(id, data)| {
                let data: T = self
                    .serializer
                    .deserialize(&*data)
                    .context(RedDbErrorKind::DataCorruption)
                    .unwrap();
                Document::new(*id, data)
            })
            .flat_map(|document| {
                self.serializer
                    .serialize(&document)
                    .context(RedDbErrorKind::DataCorruption)
                    .unwrap()
            })
            .collect();

        if self.storage_exists() {
            self.flush_data(&self.file_path, &data)?;
        }

        Ok(())
    }

    fn storage_exists(&self) -> bool {
        Path::new(&self.file_path).exists()
    }

    fn flush_data<'a, P: AsRef<Path>>(&'a self, path: P, data: &[u8]) -> Result<()> {
        let mut storage = File::create(path).context(RedDbErrorKind::DataCorruption)?;
        storage.set_len(0).context(RedDbErrorKind::FlushData)?;
        storage
            .seek(SeekFrom::Start(0))
            .context(RedDbErrorKind::FlushData)?;
        storage
            .write_all(&data)
            .context(RedDbErrorKind::FlushData)?;
        storage.sync_all().context(RedDbErrorKind::FlushData)?;
        Ok(())
    }

    fn append<'a>(&'a self, data: &[u8]) -> Result<()> {
        let mut storage = self.db_file.lock().unwrap();
        storage
            .seek(SeekFrom::End(0))
            .context(RedDbErrorKind::AppendData)?;
        storage
            .write_all(&data)
            .context(RedDbErrorKind::AppendData)?;
        storage.sync_all().context(RedDbErrorKind::AppendData)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;
    use crate::RonSerializer;
    use crate::Uuid;

    #[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
    struct TestStruct {
        foo: String,
    }

    #[test]
    fn persist_and_load_data<'a>() {
        let storage = FileStorage::<RonSerializer>::new("file_test").unwrap();
        let doc_one = Document::new(
            Uuid::new_v4(),
            TestStruct {
                foo: "one".to_owned(),
            },
        );
        let doc_two = Document::new(
            Uuid::new_v4(),
            TestStruct {
                foo: "one".to_owned(),
            },
        );
        let arr_docs = vec![doc_one.clone(), doc_two.clone()];
        let _persisted = storage.persist(&arr_docs);
        // let map: RedDbHM = storage.load::<TestStruct>().unwrap();
        // println!("{:?}", doc_one.id);
        // println!("{:?}", doc_two.id);
        // println!("{:?}eeee", map.len());
        // map.get(&doc_one.id).unwrap();
        // let one: TestStruct = storage
        //     .serializer
        //     .deserialize(&map.get(&doc_one.id).unwrap().lock().unwrap())
        //     .unwrap();
        // assert_eq!(one, doc_one.data);
    }
}
