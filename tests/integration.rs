use reddb::{DbConfig, Document, MemDb, RonDb, WriteOrder};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
struct UserRec {
    name: String,
    role: String,
}

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

// ── update_where ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_where_exec_survives_reopen() {
    let file = ".it_update_where.ron";
    cleanup(file);

    {
        let db = RonDb::new::<TestStruct>(".it_update_where").await.unwrap();
        db.insert(vec![
            TestStruct { foo: "change_me".into() },
            TestStruct { foo: "change_me".into() },
            TestStruct { foo: "keep".into() },
        ])
        .await
        .unwrap();
        let n = db
            .update_where::<TestStruct, _>(|t| t.foo == "change_me")
            .exec(|mut t| { t.foo = "changed".into(); t })
            .await
            .unwrap();
        assert_eq!(n, 2);
    }

    let db2 = RonDb::new::<TestStruct>(".it_update_where").await.unwrap();
    let changed = db2.query::<TestStruct>().filter(|t| t.foo == "changed").count().await.unwrap();
    let kept = db2.query::<TestStruct>().filter(|t| t.foo == "keep").count().await.unwrap();
    assert_eq!(changed, 2);
    assert_eq!(kept, 1);

    cleanup(file);
}

#[tokio::test]
async fn update_where_returning_gives_new_state() {
    let file = ".it_update_where_ret.ron";
    cleanup(file);

    let db = RonDb::new::<TestStruct>(".it_update_where_ret").await.unwrap();
    db.insert_one(TestStruct { foo: "before".into() }).await.unwrap();

    let docs = db
        .update_where::<TestStruct, _>(|t| t.foo == "before")
        .returning(|mut t| { t.foo = "after".into(); t })
        .await
        .unwrap();

    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].data.foo, "after");

    let found = db.find_one::<TestStruct>(&docs[0].id).await.unwrap();
    assert_eq!(found.data.foo, "after");

    cleanup(file);
}

// ── delete_where ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_where_survives_reopen() {
    let file = ".it_delete_where.ron";
    cleanup(file);

    {
        let db = RonDb::new::<TestStruct>(".it_delete_where").await.unwrap();
        db.insert(vec![
            TestStruct { foo: "remove".into() },
            TestStruct { foo: "remove".into() },
            TestStruct { foo: "keep".into() },
        ])
        .await
        .unwrap();
        let n = db.delete_where::<TestStruct, _>(|t| t.foo == "remove").await.unwrap();
        assert_eq!(n, 2);
    }

    let db2 = RonDb::new::<TestStruct>(".it_delete_where").await.unwrap();
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].data.foo, "keep");

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

#[tokio::test]
async fn manual_compact_shrinks_file() {
    let file = ".it_manual_compact.ron";
    cleanup(file);

    // Use a high ratio so auto-compaction on reopen doesn't fire (we want to
    // test the explicit compact() call, not the automatic one).
    let db: RonDb = RonDb::open::<TestStruct>(
        DbConfig::new(".it_manual_compact").compaction_ratio(100.0),
    )
    .await
    .unwrap();

    let docs = db
        .insert(vec![
            TestStruct { foo: "a".into() },
            TestStruct { foo: "b".into() },
            TestStruct { foo: "c".into() },
        ])
        .await
        .unwrap();
    db.delete_one::<TestStruct>(&docs[0].id).await.unwrap();
    db.delete_one::<TestStruct>(&docs[1].id).await.unwrap();

    let size_before = db.stats().await.unwrap().file_size_bytes;
    db.compact().await.unwrap();
    let size_after = db.stats().await.unwrap().file_size_bytes;
    assert!(size_after < size_before, "size_after={size_after} size_before={size_before}");

    let all = db.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].data.foo, "c");

    cleanup(file);
}

// ── storage stats ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn storage_stats_memdb() {
    let db = MemDb::new::<TestStruct>("unused").await.unwrap();

    let s0 = db.stats().await.unwrap();
    assert_eq!(s0.live_document_count, 0);
    assert_eq!(s0.file_size_bytes, 0);
    assert_eq!(s0.compaction_ratio, 2.0);

    db.insert(vec![
        TestStruct { foo: "x".into() },
        TestStruct { foo: "y".into() },
    ])
    .await
    .unwrap();

    let s1 = db.stats().await.unwrap();
    assert_eq!(s1.live_document_count, 2);
    assert_eq!(s1.file_size_bytes, 0);
}

#[tokio::test]
async fn storage_stats_file_size_grows() {
    let file = ".it_stats_size.ron";
    cleanup(file);

    let db = RonDb::new::<TestStruct>(".it_stats_size").await.unwrap();
    let s0 = db.stats().await.unwrap();

    db.insert_one(TestStruct { foo: "hello".into() }).await.unwrap();
    let s1 = db.stats().await.unwrap();

    assert!(s1.file_size_bytes > s0.file_size_bytes);
    assert_eq!(s1.live_document_count, 1);

    cleanup(file);
}

// ── WriteOrder::FileFirst ─────────────────────────────────────────────────────

#[tokio::test]
async fn file_first_insert_persists_across_reopen() {
    let file = ".it_file_first.ron";
    cleanup(file);

    let id = {
        let db: RonDb = RonDb::open::<TestStruct>(
            DbConfig::new(".it_file_first").write_order(WriteOrder::FileFirst),
        )
        .await
        .unwrap();
        let doc = db
            .insert_one(TestStruct { foo: "file_first".into() })
            .await
            .unwrap();
        doc.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_file_first").await.unwrap();
    let found = db2.get::<TestStruct>(&id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().data.foo, "file_first");

    cleanup(file);
}

#[tokio::test]
async fn file_first_update_where_persists_across_reopen() {
    let file = ".it_file_first_uw.ron";
    cleanup(file);

    {
        let db: RonDb = RonDb::open::<TestStruct>(
            DbConfig::new(".it_file_first_uw").write_order(WriteOrder::FileFirst),
        )
        .await
        .unwrap();
        db.insert(vec![
            TestStruct { foo: "before".into() },
            TestStruct { foo: "other".into() },
        ])
        .await
        .unwrap();
        let n = db
            .update_where::<TestStruct, _>(|t| t.foo == "before")
            .exec(|mut t| {
                t.foo = "after".into();
                t
            })
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    let db2 = RonDb::new::<TestStruct>(".it_file_first_uw").await.unwrap();
    let after_count = db2
        .query::<TestStruct>()
        .filter(|t| t.foo == "after")
        .count()
        .await
        .unwrap();
    assert_eq!(after_count, 1);

    cleanup(file);
}

// ── Transaction ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn transaction_commit_survives_reopen() {
    let file = ".it_tx_commit.ron";
    cleanup(file);

    let (inserted_id, updated_id) = {
        let db = RonDb::new::<TestStruct>(".it_tx_commit").await.unwrap();
        let existing = db.insert_one(TestStruct { foo: "original".into() }).await.unwrap();

        let mut tx = db.begin();
        let new_doc = tx.insert_one(TestStruct { foo: "tx_insert".into() }).unwrap();
        tx.update_one(&existing.id, TestStruct { foo: "tx_update".into() }).unwrap();
        tx.commit().await.unwrap();

        (new_doc.id, existing.id)
    };

    let db2 = RonDb::new::<TestStruct>(".it_tx_commit").await.unwrap();
    let inserted: Document<TestStruct> = db2.find_one(&inserted_id).await.unwrap();
    assert_eq!(inserted.data.foo, "tx_insert");
    let updated: Document<TestStruct> = db2.find_one(&updated_id).await.unwrap();
    assert_eq!(updated.data.foo, "tx_update");

    cleanup(file);
}

#[tokio::test]
async fn transaction_rollback_leaves_state_unchanged() {
    let db = MemDb::new::<TestStruct>("unused").await.unwrap();
    let doc = db.insert_one(TestStruct { foo: "original".into() }).await.unwrap();

    let mut tx = db.begin();
    tx.insert_one(TestStruct { foo: "phantom".into() }).unwrap();
    tx.update_one(&doc.id, TestStruct { foo: "changed".into() }).unwrap();
    tx.rollback();

    let all = db.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].data.foo, "original");
}

#[tokio::test]
async fn transaction_delete_survives_reopen() {
    let file = ".it_tx_delete.ron";
    cleanup(file);

    let keep_id = {
        let db = RonDb::new::<TestStruct>(".it_tx_delete").await.unwrap();
        let d1 = db.insert_one(TestStruct { foo: "delete_me".into() }).await.unwrap();
        let d2 = db.insert_one(TestStruct { foo: "keep_me".into() }).await.unwrap();

        let mut tx = db.begin();
        tx.delete_one(&d1.id);
        tx.commit().await.unwrap();

        d2.id
    };

    let db2 = RonDb::new::<TestStruct>(".it_tx_delete").await.unwrap();
    let all = db2.find_all::<TestStruct>().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, keep_id);

    cleanup(file);
}

// ── Cross-serializer persistence round-trip ───────────────────────────────────
//
// Each serializer gets the same four tests: insert / update / delete / compact,
// all checked by dropping the DB and reopening. The macro is gated per feature.

macro_rules! serializer_round_trip_tests {
    ($mod_name:ident, $DbType:ident, $ext:literal, $feature:literal) => {
        #[cfg(feature = $feature)]
        mod $mod_name {
            use super::*;
            use reddb::$DbType;

            #[tokio::test]
            async fn insert_survives_reopen() {
                let stem = concat!(".ser_", stringify!($mod_name), "_insert");
                let file = concat!(".ser_", stringify!($mod_name), "_insert.", $ext);
                cleanup(file);
                let id = {
                    let db = $DbType::new::<TestStruct>(stem).await.unwrap();
                    db.insert_one(TestStruct { foo: "hello".into() }).await.unwrap().id
                };
                let db2 = $DbType::new::<TestStruct>(stem).await.unwrap();
                assert_eq!(db2.find_one::<TestStruct>(&id).await.unwrap().data.foo, "hello");
                cleanup(file);
            }

            #[tokio::test]
            async fn update_survives_reopen() {
                let stem = concat!(".ser_", stringify!($mod_name), "_update");
                let file = concat!(".ser_", stringify!($mod_name), "_update.", $ext);
                cleanup(file);
                let id = {
                    let db = $DbType::new::<TestStruct>(stem).await.unwrap();
                    let doc = db.insert_one(TestStruct { foo: "old".into() }).await.unwrap();
                    db.update_one(&doc.id, TestStruct { foo: "new".into() }).await.unwrap();
                    doc.id
                };
                let db2 = $DbType::new::<TestStruct>(stem).await.unwrap();
                assert_eq!(db2.find_one::<TestStruct>(&id).await.unwrap().data.foo, "new");
                cleanup(file);
            }

            #[tokio::test]
            async fn delete_survives_reopen() {
                let stem = concat!(".ser_", stringify!($mod_name), "_delete");
                let file = concat!(".ser_", stringify!($mod_name), "_delete.", $ext);
                cleanup(file);
                let id = {
                    let db = $DbType::new::<TestStruct>(stem).await.unwrap();
                    let doc = db.insert_one(TestStruct { foo: "bye".into() }).await.unwrap();
                    db.delete_one::<TestStruct>(&doc.id).await.unwrap();
                    doc.id
                };
                let db2 = $DbType::new::<TestStruct>(stem).await.unwrap();
                assert!(db2.get::<TestStruct>(&id).await.unwrap().is_none());
                cleanup(file);
            }

            #[tokio::test]
            async fn compaction_survives_reopen() {
                let stem = concat!(".ser_", stringify!($mod_name), "_compact");
                let file = concat!(".ser_", stringify!($mod_name), "_compact.", $ext);
                cleanup(file);
                {
                    let db = $DbType::new::<TestStruct>(stem).await.unwrap();
                    let a = db.insert_one(TestStruct { foo: "a".into() }).await.unwrap();
                    db.insert_one(TestStruct { foo: "b".into() }).await.unwrap();
                    db.delete_one::<TestStruct>(&a.id).await.unwrap();
                }
                let db2 = $DbType::new::<TestStruct>(stem).await.unwrap();
                let all = db2.find_all::<TestStruct>().await.unwrap();
                assert_eq!(all.len(), 1);
                assert_eq!(all[0].data.foo, "b");
                cleanup(file);
            }
        }
    };
}

serializer_round_trip_tests!(bin,  BinDb,  "bin",  "bin_ser");
serializer_round_trip_tests!(json, JsonDb, "json", "json_ser");
serializer_round_trip_tests!(ron,  RonDb,  "ron",  "ron_ser");
serializer_round_trip_tests!(yaml, YamlDb, "yaml", "yaml_ser");

// ── HashIndex ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn hash_index_lookup_persisted_data() {
    let file = ".it_index.ron";
    cleanup(file);

    {
        let db = RonDb::new::<UserRec>(".it_index").await.unwrap();
        db.insert(vec![
            UserRec { name: "alice".into(), role: "admin".into() },
            UserRec { name: "bob".into(),   role: "user".into()  },
            UserRec { name: "carol".into(), role: "admin".into() },
        ])
        .await
        .unwrap();
    }

    let db2 = RonDb::new::<UserRec>(".it_index").await.unwrap();
    db2.add_index::<UserRec, _>("by_role", |u| u.role.clone()).await.unwrap();

    let admins = db2.using_index::<UserRec>("by_role", "admin").await.unwrap();
    assert_eq!(admins.len(), 2);

    cleanup(file);
}

#[tokio::test]
async fn hash_index_stays_consistent_after_mutations() {
    let db = MemDb::new::<UserRec>("unused").await.unwrap();
    db.add_index::<UserRec, _>("by_role", |u| u.role.clone()).await.unwrap();

    let d1 = db.insert_one(UserRec { name: "alice".into(), role: "admin".into() }).await.unwrap();
    let d2 = db.insert_one(UserRec { name: "bob".into(),   role: "user".into()  }).await.unwrap();
    db.insert_one(UserRec { name: "carol".into(), role: "admin".into() }).await.unwrap();

    // Promote bob to admin
    db.update_one(&d2.id, UserRec { name: "bob".into(), role: "admin".into() }).await.unwrap();

    let admins = db.using_index::<UserRec>("by_role", "admin").await.unwrap();
    assert_eq!(admins.len(), 3);
    let users = db.using_index::<UserRec>("by_role", "user").await.unwrap();
    assert!(users.is_empty());

    // Delete alice
    db.delete_one::<UserRec>(&d1.id).await.unwrap();
    let admins_after = db.using_index::<UserRec>("by_role", "admin").await.unwrap();
    assert_eq!(admins_after.len(), 2);
}
