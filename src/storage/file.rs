use async_trait::async_trait;
use core::fmt::Debug;
use failure::ResultExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use super::Storage;
use crate::document::Document;
use crate::error::{RedDbErrorKind, Result};
use crate::serializer::{Serializer, Serializers};
use crate::status::Status;
use crate::RedDbHM;
use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, SeekFrom};

use tokio::sync::Mutex;

#[derive(Debug)]
pub struct FileStorage<SE> {
    file_path: String,
    serializer: SE,
    db_file: Mutex<File>,
}

#[async_trait]
impl<SE> Storage for FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug + Sync + Send,
{
    async fn new(db_name: &str) -> Result<Self> {
        let serializer = SE::default();
        let extension = match serializer.format() {
            Serializers::Bin(st) => st,
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
                    .await
                    .map_err(|_| RedDbErrorKind::StorageOpen)?,
            ),
        })
    }

    async fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let mut map: RedDbHM = HashMap::new();
        let mut file = self.db_file.lock().await;
        let reader = BufReader::new(&mut *file);

        let mut lines = reader.lines();
        let mut records = 0;
        let start = Instant::now();

        while let Some(line) = lines.next_line().await.unwrap() {
            let byte_str = &line.into_bytes();
            let document: Document<T> = self
                .serializer
                .deserialize(byte_str)
                .context(RedDbErrorKind::DataCorruption)
                .unwrap();
            let id = document._id;
            let st = document._st;
            let data = document.data;
            let serialized = self.serializer.serialize(&data).unwrap();
            records += 1;
            if let Status::De = st {
                map.remove(&id);
            } else {
                map.entry(id).or_insert_with(|| serialized);
            }
        }
        let duration = start.elapsed();

        println!("[RedDb] {:?} records loaded ({:?})", &records, duration);
        println!("[RedDb] Compacting data...");
        let start = Instant::now();

        self.compact_data::<T>(&map)
            .await
            .context(RedDbErrorKind::Compact)?;
        let duration = start.elapsed();

        println!(
            "[RedDb] {:?} records compacted ({:?})",
            &map.len(),
            duration
        );

        Ok(map)
    }

    async fn persist<T>(&self, data: &[Document<T>]) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Sync,
    {
        let serialized: Vec<u8> = data
            .iter()
            .flat_map(|doc| self.serializer.serialize::<Document<T>>(doc).unwrap())
            .collect();

        self.append(&serialized)
            .await
            .context(RedDbErrorKind::AppendData)?;

        Ok(())
    }
}

impl<SE> FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug,
{
    pub async fn compact_data<T>(&self, data: &RedDbHM) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data: Vec<u8> = data
            .iter()
            .flat_map(|(id, data)| {
                let data: T = self
                    .serializer
                    .deserialize(&*data)
                    .context(RedDbErrorKind::DataCorruption)
                    .unwrap();

                self.serializer
                    .serialize(&Document::new(*id, data, Status::In))
                    .context(RedDbErrorKind::DataCorruption)
                    .unwrap()
            })
            .collect();

        self.flush_data(&self.file_path, &data).await.unwrap();

        Ok(())
    }

    /*
    fn storage_exists(&self) -> bool {
        Path::new(&self.file_path).exists()
    }*/

    async fn flush_data<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> Result<()> {
        let mut storage = File::create(path)
            .await
            .context(RedDbErrorKind::DataCorruption)?;
        storage
            .set_len(0)
            .await
            .context(RedDbErrorKind::FlushData)?;
        storage
            .seek(SeekFrom::Start(0))
            .await
            .context(RedDbErrorKind::FlushData)?;
        storage
            .write_all(&data)
            .await
            .context(RedDbErrorKind::FlushData)?;
        storage
            .sync_all()
            .await
            .context(RedDbErrorKind::FlushData)?;
        Ok(())
    }

    async fn append(&self, data: &[u8]) -> Result<()> {
        let mut storage = self.db_file.lock().await;
        storage.seek(SeekFrom::End(0)).await.unwrap();
        storage.write_all(&data).await.unwrap();
        storage.sync_all().await.unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use crate::serializer::RonSerializer;
    use crate::Document;
    use crate::Uuid;

    #[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
    struct TestStruct {
        foo: String,
    }

    // #[tokio::test]
    // async fn persist_and_load_data<'a>() {
    //     let storage = FileStorage::<RonSerializer>::new("file_persist_test")
    //         .await
    //         .unwrap();
    //     let doc_one = Document::new(
    //         Uuid::new_v4(),
    //         TestStruct {
    //             foo: "one".to_owned(),
    //         },
    //         Status::In,
    //     );
    //     let doc_two = Document::new(
    //         Uuid::new_v4(),
    //         TestStruct {
    //             foo: "one".to_owned(),
    //         },
    //         Status::In,
    //     );
    //     let arr_docs = vec![doc_one.clone(), doc_two.clone()];
    //     let _persisted = storage.persist(&arr_docs).await.unwrap();
    //     let map: RedDbHM = storage.load::<TestStruct>().await.unwrap();
    //     let one: TestStruct = storage
    //         .serializer
    //         .deserialize(&map.get(&doc_one._id).unwrap())
    //         .unwrap();
    //     // assert_eq!(one, doc_one.data);
    // }
}
