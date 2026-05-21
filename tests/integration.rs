use reddb::{Document, RonDb};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
struct TestStruct {
    foo: String,
}

/// Remove the file at `path`, ignoring errors (e.g. does not exist).
fn cleanup(path: &str) {
    let _ = fs::remove_file(path);
}

// ── insert ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn insert_one_survives_reopen() {
    let file = ".it_insert_one.ron";
    cleanup(file);

    let inserted_id = {
        let db = RonDb::new::<TestStruct>(".it_insert_one").unwrap();
        let doc = db
            .insert_one(TestStruct { foo: "persist_me".into() })
            .await
            .unwrap();
        doc.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_insert_one").unwrap();
    let found: Option<Document<TestStruct>> = db2.get(&inserted_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().data.foo, "persist_me");

    cleanup(file);
}

#[tokio::test]
async fn insert_many_survives_reopen() {
    let file = ".it_insert_many.ron";
    cleanup(file);

    let ids: Vec<_> = {
        let db = RonDb::new::<TestStruct>(".it_insert_many").unwrap();
        db.insert(vec![
            TestStruct { foo: "alpha".into() },
            TestStruct { foo: "beta".into() },
            TestStruct { foo: "gamma".into() },
        ])
        .await
        .unwrap()
        .into_iter()
        .map(|d| d.id)
        .collect()
    };

    let db2 = RonDb::new::<TestStruct>(".it_insert_many").unwrap();
    for id in &ids {
        assert!(db2.get::<TestStruct>(id).await.unwrap().is_some());
    }
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 3);

    cleanup(file);
}

// ── update ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_one_survives_reopen() {
    let file = ".it_update_one.ron";
    cleanup(file);

    let id = {
        let db = RonDb::new::<TestStruct>(".it_update_one").unwrap();
        let doc = db
            .insert_one(TestStruct { foo: "original".into() })
            .await
            .unwrap();
        db.update_one(&doc.id, TestStruct { foo: "updated".into() })
            .await
            .unwrap();
        doc.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_update_one").unwrap();
    let found: Document<TestStruct> = db2.find_one(&id).await.unwrap();
    assert_eq!(found.data.foo, "updated");

    cleanup(file);
}

#[tokio::test]
async fn update_many_survives_reopen() {
    let file = ".it_update_many.ron";
    cleanup(file);

    let search = TestStruct { foo: "search".into() };
    let replacement = TestStruct { foo: "replaced".into() };

    {
        let db = RonDb::new::<TestStruct>(".it_update_many").unwrap();
        db.insert(vec![
            search.clone(),
            search.clone(),
            TestStruct { foo: "other".into() },
        ])
        .await
        .unwrap();
        let n = db.update(&search, &replacement).await.unwrap();
        assert_eq!(n, 2);
    }

    let db2 = RonDb::new::<TestStruct>(".it_update_many").unwrap();
    let replaced = db2.find(&replacement).await.unwrap();
    assert_eq!(replaced.len(), 2);
    let untouched = db2.find(&TestStruct { foo: "other".into() }).await.unwrap();
    assert_eq!(untouched.len(), 1);

    cleanup(file);
}

// ── delete ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_one_survives_reopen() {
    let file = ".it_delete_one.ron";
    cleanup(file);

    let id = {
        let db = RonDb::new::<TestStruct>(".it_delete_one").unwrap();
        let doc = db
            .insert_one(TestStruct { foo: "to_delete".into() })
            .await
            .unwrap();
        db.delete_one::<TestStruct>(&doc.id).await.unwrap();
        doc.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_delete_one").unwrap();
    let result = db2.get::<TestStruct>(&id).await.unwrap();
    assert!(result.is_none());

    cleanup(file);
}

#[tokio::test]
async fn delete_many_survives_reopen() {
    let file = ".it_delete_many.ron";
    cleanup(file);

    let target = TestStruct { foo: "remove_me".into() };
    let keep = TestStruct { foo: "keep_me".into() };

    {
        let db = RonDb::new::<TestStruct>(".it_delete_many").unwrap();
        db.insert(vec![target.clone(), target.clone(), keep.clone()])
            .await
            .unwrap();
        let n = db.delete(&target).await.unwrap();
        assert_eq!(n, 2);
    }

    let db2 = RonDb::new::<TestStruct>(".it_delete_many").unwrap();
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].data.foo, "keep_me");

    cleanup(file);
}

// ── compaction ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn compaction_produces_correct_state() {
    let file = ".it_compaction.ron";
    cleanup(file);

    {
        let db = RonDb::new::<TestStruct>(".it_compaction").unwrap();
        // Three inserts + one delete = net two live docs
        let a = db
            .insert_one(TestStruct { foo: "a".into() })
            .await
            .unwrap();
        let _b = db.insert_one(TestStruct { foo: "b".into() }).await.unwrap();
        let _c = db.insert_one(TestStruct { foo: "c".into() }).await.unwrap();
        db.delete_one::<TestStruct>(&a.id).await.unwrap();
    }

    // Reopening triggers compaction; final state should have 2 docs
    let db2 = RonDb::new::<TestStruct>(".it_compaction").unwrap();
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 2);
    let foos: Vec<&str> = all.iter().map(|d| d.data.foo.as_str()).collect();
    assert!(foos.contains(&"b"));
    assert!(foos.contains(&"c"));

    cleanup(file);
}
