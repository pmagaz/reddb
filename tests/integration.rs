use reddb::{Document, RonDb};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
struct TestStruct {
    foo: String,
}

#[tokio::test]
async fn insert_one_and_persist() {
    let db = RonDb::new::<TestStruct>(".insert_one_persist.db").unwrap();
    let doc: Document<TestStruct> = db
        .insert_one(TestStruct { foo: "test".to_owned() })
        .await
        .unwrap();

    let file = File::open(".insert_one_persist.db.ron").unwrap();
    let buffered = BufReader::new(file);

    for line in buffered.lines() {
        let bytes = line.unwrap().into_bytes();
        // The file stores StorageDoc (internal envelope); parse the id and data fields
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains(&doc.id.to_string()));
        assert!(text.contains("test"));
    }
    fs::remove_file(".insert_one_persist.db.ron").unwrap();
}

#[tokio::test]
async fn insert_and_persist() {
    let db = RonDb::new::<TestStruct>(".insert_persist.db").unwrap();
    let one = TestStruct { foo: "one".to_owned() };
    let two = TestStruct { foo: "two".to_owned() };
    let inserted: Vec<Document<TestStruct>> = db
        .insert(vec![one.clone(), two.clone()])
        .await
        .unwrap();

    let file = File::open(".insert_persist.db.ron").unwrap();
    let buffered = BufReader::new(file);
    let lines: Vec<String> = buffered.lines().map(|l| l.unwrap()).collect();

    assert_eq!(lines.len(), 2);
    assert!(lines.iter().any(|l| l.contains("one")));
    assert!(lines.iter().any(|l| l.contains("two")));
    assert!(lines.iter().any(|l| l.contains(&inserted[0].id.to_string())));
    assert!(lines.iter().any(|l| l.contains(&inserted[1].id.to_string())));

    fs::remove_file(".insert_persist.db.ron").unwrap();
}

#[tokio::test]
async fn update_one_and_persist() {
    let db = RonDb::new::<TestStruct>(".update_one_persist.db").unwrap();
    let doc: Document<TestStruct> = db
        .insert_one(TestStruct { foo: "original".to_owned() })
        .await
        .unwrap();

    db.update_one(&doc.id, TestStruct { foo: "updated".to_owned() })
        .await
        .unwrap();

    let result: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
    assert_eq!(result.data.foo, "updated");

    let file = File::open(".update_one_persist.db.ron").unwrap();
    let buffered = BufReader::new(file);
    let lines: Vec<String> = buffered.lines().map(|l| l.unwrap()).collect();
    // After compaction on load there is 1 line; before compaction there are 2
    // Either way the last occurrence of this id must have "updated"
    let id_str = doc.id.to_string();
    let last = lines.iter().filter(|l| l.contains(&id_str)).last().unwrap();
    assert!(last.contains("updated"));

    fs::remove_file(".update_one_persist.db.ron").unwrap();
}

#[tokio::test]
async fn update_and_persist() {
    let db = RonDb::new::<TestStruct>(".update_persist.db").unwrap();
    let one = TestStruct { foo: "search".to_owned() };
    let two = TestStruct { foo: "other".to_owned() };
    let updated_val = TestStruct { foo: "updated".to_owned() };

    db.insert(vec![one.clone(), one.clone(), two.clone()])
        .await
        .unwrap();

    let num_updated = db.update(&one, &updated_val).await.unwrap();
    assert_eq!(num_updated, 2);

    let results = db.find(&updated_val).await.unwrap();
    assert_eq!(results.len(), 2);

    fs::remove_file(".update_persist.db.ron").unwrap();
}

#[tokio::test]
async fn delete_one_persists_removal() {
    let db = RonDb::new::<TestStruct>(".delete_one_persist.db").unwrap();
    let doc = db
        .insert_one(TestStruct { foo: "to_delete".to_owned() })
        .await
        .unwrap();

    db.delete_one::<TestStruct>(&doc.id).await.unwrap();

    let after = db.get::<TestStruct>(&doc.id).await.unwrap();
    assert!(after.is_none());

    fs::remove_file(".delete_one_persist.db.ron").unwrap();
}
