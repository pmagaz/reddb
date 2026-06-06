use reddb::{Document, MemDb, RonDb};
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
        let db = RonDb::new::<TestStruct>(".it_insert_one").await.unwrap();
        let doc = db
            .insert_one(TestStruct { foo: "persist_me".into() })
            .await
            .unwrap();
        doc.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_insert_one").await.unwrap();
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
        let db = RonDb::new::<TestStruct>(".it_insert_many").await.unwrap();
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

    let db2 = RonDb::new::<TestStruct>(".it_insert_many").await.unwrap();
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
        let db = RonDb::new::<TestStruct>(".it_update_one").await.unwrap();
        let doc = db
            .insert_one(TestStruct { foo: "original".into() })
            .await
            .unwrap();
        db.update_one(&doc.id, TestStruct { foo: "updated".into() })
            .await
            .unwrap();
        doc.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_update_one").await.unwrap();
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
        let db = RonDb::new::<TestStruct>(".it_update_many").await.unwrap();
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

    let db2 = RonDb::new::<TestStruct>(".it_update_many").await.unwrap();
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
        let db = RonDb::new::<TestStruct>(".it_delete_one").await.unwrap();
        let doc = db
            .insert_one(TestStruct { foo: "to_delete".into() })
            .await
            .unwrap();
        db.delete_one::<TestStruct>(&doc.id).await.unwrap();
        doc.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_delete_one").await.unwrap();
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
        let db = RonDb::new::<TestStruct>(".it_delete_many").await.unwrap();
        db.insert(vec![target.clone(), target.clone(), keep.clone()])
            .await
            .unwrap();
        let n = db.delete(&target).await.unwrap();
        assert_eq!(n, 2);
    }

    let db2 = RonDb::new::<TestStruct>(".it_delete_many").await.unwrap();
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].data.foo, "keep_me");

    cleanup(file);
}

// ── MemDb ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn memdb_basic_crud() {
    let db = MemDb::new::<TestStruct>("unused").await.unwrap();

    let doc = db.insert_one(TestStruct { foo: "hello".into() }).await.unwrap();
    let found: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
    assert_eq!(found.data.foo, "hello");

    db.update_one(&doc.id, TestStruct { foo: "world".into() }).await.unwrap();
    let updated: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
    assert_eq!(updated.data.foo, "world");

    let deleted: Document<TestStruct> = db.delete_one(&doc.id).await.unwrap();
    assert_eq!(deleted.id, doc.id);
    assert!(db.get::<TestStruct>(&doc.id).await.unwrap().is_none());
}

#[tokio::test]
async fn memdb_leaves_no_files() {
    let stem = ".it_memdb_no_files";
    let _ = fs::remove_file(format!("{}.bin", stem));

    let db = MemDb::new::<TestStruct>(stem).await.unwrap();
    db.insert_one(TestStruct { foo: "ephemeral".into() }).await.unwrap();
    drop(db);

    assert!(!std::path::Path::new(&format!("{}.bin", stem)).exists());
}

#[tokio::test]
async fn memdb_does_not_persist_across_reopen() {
    let db1 = MemDb::new::<TestStruct>("memdb_ephemeral").await.unwrap();
    db1.insert_one(TestStruct { foo: "gone".into() }).await.unwrap();
    drop(db1);

    let db2 = MemDb::new::<TestStruct>("memdb_ephemeral").await.unwrap();
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert!(all.is_empty());
}

// ── query ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn query_filter_survives_reopen() {
    let file = ".it_query_filter.ron";
    cleanup(file);

    {
        let db = RonDb::new::<TestStruct>(".it_query_filter").await.unwrap();
        db.insert(vec![
            TestStruct { foo: "keep".into() },
            TestStruct { foo: "keep".into() },
            TestStruct { foo: "drop".into() },
        ])
        .await
        .unwrap();
    }

    let db2 = RonDb::new::<TestStruct>(".it_query_filter").await.unwrap();
    let results = db2
        .query::<TestStruct>()
        .filter(|t| t.foo == "keep")
        .all()
        .await
        .unwrap();
    assert_eq!(results.len(), 2);

    cleanup(file);
}

#[tokio::test]
async fn query_order_limit_skip() {
    let file = ".it_query_chain.ron";
    cleanup(file);

    {
        let db = RonDb::new::<TestStruct>(".it_query_chain").await.unwrap();
        db.insert(vec![
            TestStruct { foo: "c".into() },
            TestStruct { foo: "a".into() },
            TestStruct { foo: "b".into() },
            TestStruct { foo: "d".into() },
        ])
        .await
        .unwrap();
    }

    let db2 = RonDb::new::<TestStruct>(".it_query_chain").await.unwrap();
    // sorted asc, skip first, take 2 → "b", "c"
    let results = db2
        .query::<TestStruct>()
        .order_by(|a, b| a.foo.cmp(&b.foo))
        .skip(1)
        .limit(2)
        .all()
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].data.foo, "b");
    assert_eq!(results[1].data.foo, "c");

    cleanup(file);
}

#[tokio::test]
async fn query_count_and_ids() {
    let file = ".it_query_count.ron";
    cleanup(file);

    let db = RonDb::new::<TestStruct>(".it_query_count").await.unwrap();
    db.insert(vec![
        TestStruct { foo: "yes".into() },
        TestStruct { foo: "yes".into() },
        TestStruct { foo: "no".into() },
    ])
    .await
    .unwrap();

    let count = db.query::<TestStruct>().filter(|t| t.foo == "yes").count().await.unwrap();
    assert_eq!(count, 2);

    let ids = db.query::<TestStruct>().filter(|t| t.foo == "yes").ids().await.unwrap();
    assert_eq!(ids.len(), 2);

    cleanup(file);
}

// ── compaction ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn compaction_produces_correct_state() {
    let file = ".it_compaction.ron";
    cleanup(file);

    {
        let db = RonDb::new::<TestStruct>(".it_compaction").await.unwrap();
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
    let db2 = RonDb::new::<TestStruct>(".it_compaction").await.unwrap();
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 2);
    let foos: Vec<&str> = all.iter().map(|d| d.data.foo.as_str()).collect();
    assert!(foos.contains(&"b"));
    assert!(foos.contains(&"c"));

    cleanup(file);
}
