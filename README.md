# RedDB

[![Actions Status](https://github.com/pmagaz/reddb/workflows/build/badge.svg)](https://github.com/pmagaz/reddb/actions) [![Crates.io](https://img.shields.io/crates/v/reddb)](https://crates.io/crates/reddb)

An async, in-memory embedded document database for Rust with optional WAL-based persistence.

## Features

- **In-memory first** — the live store is an `Arc<RwLock<HashMap<Uuid, Vec<u8>>>>`. Every read and write hits RAM; disk is never on the hot path.
- **Optional persistence** — a WAL-style append-only log survives process restarts. Choose `MemDb` for pure in-memory operation or a typed alias (`BinDb`, `JsonDb`, `RonDb`, `YamlDb`) for durability.
- **Async-first** — built on Tokio 1.x; every I/O method is `async`.
- **Pluggable serializers** — Binary (bincode), JSON, RON, and YAML, each behind an optional feature flag.
- **Closure-based queries** — `QueryBuilder` with `.filter()`, `.order_by()`, `.skip()`, `.limit()`, terminating with `.all()`, `.first()`, `.count()`, or `.ids()`.
- **Bulk updates and deletes** — `update_where` and `delete_where` accept arbitrary predicates.
- **Transactions** — `begin()` / `commit()` / `rollback()` buffer operations and apply them atomically.
- **Hash indexes** — `add_index` registers a string-keyed index maintained automatically on every write; `using_index` looks up documents in O(1).
- **Compaction** — `compact()` rewrites the log with exactly one record per live document.
- **Configurable write order** — `MemoryFirst` (default, faster) or `FileFirst` (stronger durability guarantee).

---

## Quick start

```rust
use reddb::{MemDb, Document};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Note {
    title: String,
    body: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = MemDb::new::<Note>("notes").await?;

    // Insert one document
    let doc: Document<Note> = db.insert_one(Note {
        title: "Hello".into(),
        body: "World".into(),
    }).await?;

    // Retrieve by id
    let found: Document<Note> = db.find_one(&doc.id).await?;
    println!("{}", found.data.title);

    // Update by id
    db.update_one(&doc.id, Note {
        title: "Hello".into(),
        body: "Updated".into(),
    }).await?;

    // Delete by id
    db.delete_one::<Note>(&doc.id).await?;

    Ok(())
}
```

Batch insert and equality-based operations:

```rust
// Batch insert
let docs = db.insert(vec![
    Note { title: "A".into(), body: "one".into() },
    Note { title: "A".into(), body: "two".into() },
]).await?;

// Find all documents equal to a value (exact match on serialized bytes)
let matches: Vec<Document<Note>> = db.find(&Note {
    title: "A".into(),
    body: "one".into(),
}).await?;

// Update all documents equal to a value; returns the count of updated docs
let updated = db.update(
    &Note { title: "A".into(), body: "one".into() },
    &Note { title: "A".into(), body: "replaced".into() },
).await?;

// Delete all documents equal to a value; returns the count of deleted docs
let deleted = db.delete(&Note { title: "A".into(), body: "two".into() }).await?;

// Retrieve all
let all: Vec<Document<Note>> = db.find_all().await?;
```

---

## Persistence

Use a serializer-typed alias to enable file persistence. The database appends to a WAL on every write and reloads the full state on `open` or `new`.

```rust
use reddb::{RonDb, Document};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Note { title: String, body: String }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // First run: create and populate
    {
        let db = RonDb::new::<Note>("notes").await?;
        db.insert_one(Note {
            title: "Persisted".into(),
            body: "survives restart".into(),
        }).await?;
    } // db is dropped; file is flushed

    // Second run: reopen and verify
    let db = RonDb::new::<Note>("notes").await?;
    let all: Vec<Document<Note>> = db.find_all().await?;
    assert!(!all.is_empty());

    Ok(())
}
```

The file is named `<db_name><extension>` (e.g. `notes.ron`) in the current directory by default. Use `DbConfig` to change the location — see [Configuration](#configuration).

---

## Queries

`QueryBuilder` provides a lazy, chainable interface. Execution happens only when a terminal method is called.

```rust
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Task {
    priority: u32,
    done: bool,
}

// All pending tasks, sorted by priority descending, second page of 10
let results: Vec<Document<Task>> = db
    .query::<Task>()
    .filter(|t| !t.done)
    .order_by(|a, b| b.priority.cmp(&a.priority))
    .skip(10)
    .limit(10)
    .all()
    .await?;

// First matching document
let first: Option<Document<Task>> = db
    .query::<Task>()
    .filter(|t| t.priority > 5)
    .first()
    .await?;

// Count only
let n: usize = db.query::<Task>().filter(|t| t.done).count().await?;

// Only IDs
use reddb::Uuid;
let ids: Vec<Uuid> = db.query::<Task>().filter(|t| !t.done).ids().await?;
```

---

## update_where

`update_where` selects documents by predicate and applies a transformation closure. Use `.exec()` to get the count of updated documents or `.returning()` to get the updated documents back. An optional `.limit()` caps the number of documents affected.

```rust
// Boost the priority of all pending tasks by 1; returns the count
let count: usize = db
    .update_where::<Task, _>(|t| !t.done)
    .exec(|mut t| { t.priority += 1; t })
    .await?;

// Cap at 5 updates and return the modified documents
let docs: Vec<Document<Task>> = db
    .update_where::<Task, _>(|t| t.priority == 0)
    .limit(5)
    .returning(|mut t| { t.priority = 1; t })
    .await?;
```

---

## delete_where

`delete_where` removes every document that satisfies a predicate and returns the count of deleted documents.

```rust
// Remove all completed tasks
let deleted: usize = db
    .delete_where::<Task, _>(|t| t.done)
    .await?;
```

---

## Transactions

`begin()` returns a `Transaction` that buffers operations. The live store is not modified until `commit()` is called. `rollback()` silently discards all staged operations.

```rust
let mut tx = db.begin();

let doc = tx.insert_one(Task { priority: 3, done: false })?;
tx.update_one(&existing_id, Task { priority: 5, done: false })?;
tx.delete_one(&stale_id);

// Apply atomically — in-memory map, indexes, and WAL are updated together
tx.commit().await?;

// Or discard everything without touching the store
// tx.rollback();
```

Staged operations are not visible to concurrent readers until `commit` returns successfully.

---

## Hash indexes

`add_index` builds a string-keyed hash index over the existing collection and keeps it current on every subsequent write. `using_index` performs an O(1) key lookup.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct User {
    name: String,
    role: String,
}

// Register the index (scans all existing documents once to build the initial state)
db.add_index::<User, _>("by_role", |u| u.role.clone()).await?;

// O(1) lookup — returns all documents whose role field equals "admin"
let admins: Vec<Document<User>> = db.using_index::<User>("by_role", "admin").await?;

// The index is maintained automatically on every insert, update, and delete
db.insert_one(User { name: "dave".into(), role: "admin".into() }).await?;
// admins index now includes dave
```

Multiple named indexes may be registered on the same database instance. Lookups on an unregistered index name return an error.

---

## Configuration

`DbConfig` is a builder that controls how a database is opened or created.

```rust
use reddb::{RonDb, DbConfig, WriteOrder};

let db = RonDb::open::<Note>(
    DbConfig::new("notes")
        .dir("/var/lib/myapp")
        .compaction_ratio(3.0)
        .write_order(WriteOrder::FileFirst),
).await?;
```

| Option | Default | Description |
|---|---|---|
| `dir(path)` | `.` (current directory) | Directory where the WAL file is written |
| `compaction_ratio(f64)` | `2.0` | Compact when file size >= live data size × ratio |
| `write_order(WriteOrder)` | `MemoryFirst` | Order of in-memory and WAL updates on each write |

### WriteOrder

- **`MemoryFirst`** (default) — updates the in-memory map first, then appends to the WAL. Lowest latency. A crash between the two steps leaves the WAL one record behind, which is self-correcting on next open.
- **`FileFirst`** — appends to the WAL first, then updates the in-memory map. Stronger durability guarantee: if the process crashes after the WAL write, the in-memory state is reconstructed correctly on restart.

---

## Storage stats

`stats()` returns a point-in-time snapshot of storage metrics.

```rust
let s = db.stats().await?;
println!("documents : {}", s.live_document_count);
println!("file bytes: {}", s.file_size_bytes);   // always 0 for MemDb
println!("ratio     : {}", s.compaction_ratio);
```

Trigger a manual compaction to rewrite the log with one record per live document:

```rust
db.compact().await?;
```

`compact()` is a no-op for `MemDb`.

---

## Serializers

Each serializer is gated behind a Cargo feature flag. Enable only the formats you need, or use `full` to enable all of them.

| Feature flag | Type alias | Format | File extension |
|---|---|---|---|
| `bin_ser` | `BinDb` | bincode (binary) | `.bin` |
| `bin_ser` | `MemDb` | bincode (in-memory only, no file) | — |
| `json_ser` | `JsonDb` | JSON | `.json` |
| `ron_ser` | `RonDb` | RON | `.ron` |
| `yaml_ser` | `YamlDb` | YAML | `.yaml` |

All type aliases expand to `RedDb<Serializer, Storage>`. You can compose your own combination by naming the type parameters directly if you need a custom serializer or storage backend.

---

## Cargo.toml

```toml
[dependencies]
reddb = { version = "2.0", features = ["ron_ser"] }
tokio  = { version = "1",  features = ["macros", "rt-multi-thread"] }
serde  = { version = "1",  features = ["derive"] }
```

To enable all serializers:

```toml
[dependencies]
reddb = { version = "2.0", features = ["full"] }
```

---

## License

RedDB is dual-licensed under MIT or Apache-2.0, at your option.

- [MIT](https://github.com/pmagaz/reddb/blob/master/LICENSE-MIT)
- [Apache-2.0](https://opensource.org/licenses/Apache-2.0)
