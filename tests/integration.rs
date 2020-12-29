use reddb::{Document, RonDb};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Error;
use std::io::{BufRead, BufReader};
use std::path::Path;
use uuid::Uuid;

type Result<T, E = Error> = std::result::Result<T, E>;

async fn setup() -> Result<()> {
    if Path::new(".db.yaml").exists() {
        fs::remove_file(".db.yaml").unwrap();
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
struct TestStruct {
    foo: String,
}

#[tokio::test]
async fn insert_one_and_persist<'a>() {
    let db = RonDb::new::<TestStruct>(".insert_one_persist.db").unwrap();
    let doc: Document<TestStruct> = db
        .insert_one(TestStruct {
            foo: "test".to_owned(),
        })
        .await
        .unwrap();

    let file = File::open(".insert_one_persist.db.ron").unwrap();
    let buffered = BufReader::new(file);

    for line in buffered.lines() {
        let byte_str = &line.unwrap().into_bytes();
        let persisted: Document<TestStruct> = ron::de::from_bytes(byte_str).unwrap();
        assert_eq!(doc, persisted);
    }
    fs::remove_file(".insert_one_persist.db.ron").unwrap();
}

#[tokio::test]
async fn insert_and_persist<'a>() {
    let db = RonDb::new::<TestStruct>(".insert_persist.db").unwrap();
    let one = TestStruct {
        foo: "one".to_owned(),
    };
    let two = TestStruct {
        foo: "two".to_owned(),
    };
    let arr_docs = vec![one.clone(), two.clone()];
    let inserted: Vec<Document<TestStruct>> = db.insert(arr_docs).await.unwrap();
    let file = File::open(".insert_persist.db.ron").unwrap();
    let buffered = BufReader::new(file);
    for line in buffered.lines() {
        let byte_str = &line.unwrap().into_bytes();
        let persisted: Document<TestStruct> = ron::de::from_bytes(byte_str).unwrap();
        assert_eq!(inserted.contains(&persisted), true);
    }
    fs::remove_file(".insert_persist.db.ron").unwrap();
}

#[tokio::test]
async fn update_one_and_persist<'a>() {
    let db = RonDb::new::<TestStruct>(".update_one_persist.db").unwrap();
    let doc: Document<TestStruct> = db
        .insert_one(TestStruct {
            foo: "test".to_owned(),
        })
        .await
        .unwrap();
    let update = TestStruct {
        foo: "updated".to_owned(),
    };
    db.update_one(&doc._id, update.clone()).await.unwrap();
    let updated: Document<TestStruct> = db.find_one(&doc._id).await.unwrap();
    let file = File::open(".update_one_persist.db.ron").unwrap();
    let buffered = BufReader::new(file);
    let mut key = 1;
    for line in buffered.lines() {
        let byte_str = &line.unwrap().into_bytes();
        let persisted: Document<TestStruct> = ron::de::from_bytes(byte_str).unwrap();
        match key {
            1 => assert_eq!(doc, persisted),
            // 2 => assert_eq!(persisted, updated),
            _ => println!("Woops!"),
        }
        key += key;
    }
    fs::remove_file(".update_one_persist.db.ron").unwrap();
}

#[tokio::test]
async fn update_and_persist<'a>() {
    let db = RonDb::new::<TestStruct>(".update_persist.db").unwrap();
    let one = TestStruct {
        foo: "search".to_owned(),
    };
    let two = TestStruct {
        foo: "other".to_owned(),
    };
    let updated = TestStruct {
        foo: "updated".to_owned(),
    };
    let arr_docs = vec![one.clone(), one.clone(), two.clone()];
    let inserted: Vec<Document<TestStruct>> = db.insert(arr_docs).await.unwrap();
    let num_updated = db.update(&one, &updated).await.unwrap();
    assert_eq!(num_updated, 2);

    let file = File::open(".update_persist.db.ron").unwrap();
    let buffered = BufReader::new(file);
    let mut key = 0;
    let mut arr_ids: Vec<Uuid> = vec![];
    for line in buffered.lines() {
        let byte_str = &line.unwrap().into_bytes();
        let persisted: Document<TestStruct> = ron::de::from_bytes(byte_str).unwrap();
        match key {
            0 => arr_ids.push(persisted._id),
            1 => arr_ids.push(persisted._id),
            4 => {
                assert_eq!(persisted.data, updated);
                assert_eq!(arr_ids.contains(&persisted._id), true);
            }
            5 => {
                assert_eq!(persisted.data, updated);
                assert_eq!(arr_ids.contains(&persisted._id), true);
            }
            _ => println!("Woops!"),
        }
        key += key;
    }
    fs::remove_file(".update_persist.db.ron").unwrap();
}
