# RedDB v2.0 — Design & Implementation Proposal

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Design Goals](#2-design-goals)
3. [Breaking Changes Summary](#3-breaking-changes-summary)
4. [New Data Model](#4-new-data-model)
5. [Closure-Based Query API](#5-closure-based-query-api)
6. [Persistence — Binary File Format](#6-persistence--binary-file-format)
7. [Persistence — WAL & Compaction Strategy](#7-persistence--wal--compaction-strategy)
8. [Storage Trait Redesign](#8-storage-trait-redesign)
9. [Serializer Redesign](#9-serializer-redesign)
10. [Indexing System](#10-indexing-system)
11. [Transaction Support](#11-transaction-support)
12. [Complete API Reference](#12-complete-api-reference)
13. [Error Handling](#13-error-handling)
14. [Configuration](#14-configuration)
15. [Migration Guide from v1](#15-migration-guide-from-v1)
16. [Example Code](#16-example-code)
17. [Cargo.toml Changes](#17-cargotoml-changes)
18. [Implementation Roadmap](#18-implementation-roadmap)

---

## 1. Current State Analysis

### What works well

- Pluggable serializer/storage traits — the generic architecture is sound.
- AOF (Append-Only File) append semantics give crash-safety for text formats.
- Async-first design via Tokio.
- Type-safe generic API.
- Startup compaction keeps the file clean between sessions.

### Known limitations

| # | Problem | Root Cause |
|---|---------|-----------|
| 1 | Search is exact-match only | `find()` serializes the search value and does a byte comparison against stored bytes |
| 2 | Binary format is broken | `\n` delimiter appended after bincode bytes — bincode output can contain `0x0A` bytes, corrupting line-delimited reads |
| 3 | Compaction happens on every startup | No threshold — rewrites the full file even if nothing changed |
| 4 | No range or partial queries | No way to express "age > 30" or "name starts with" |
| 5 | `db_name: &'static str` | Forces a static string; cannot build paths at runtime |
| 6 | `Status` leaked into `Document` | Operation metadata (Insert/Update/Delete) is part of the public domain type |
| 7 | No pagination or ordering | `find_all` returns everything, unsorted |
| 8 | No transactions | Multi-step operations are not atomic |
| 9 | Debug `println!` in YAML serializer | `yaml.rs:30` |
| 10 | No way to update in place | `update` requires a full replacement value, not a mutation closure |

---

## 2. Design Goals

### Core principle

RedDB is an **in-memory database first**. All reads and writes operate on a `HashMap` kept in RAM. Persistence to disk is **optional** — when enabled, every write is also journaled to a file so the in-memory state can be recovered on restart. The file is never on the hot path for reads.

### Primary goals

1. **Closure-based queries** — predicates and mutators are Rust closures; the database yields deserialized `T` values to the closure.
2. **Working persistence for all formats** — proper length-prefixed framing so binary, JSON, RON, and YAML all persist correctly.
3. **Smarter compaction** — threshold-based, not startup-forced; also triggerable manually.
4. **Clean document model** — `Status` removed from `Document<T>`, kept as internal WAL entry type.
5. **Runtime-configurable paths** — `String` instead of `&'static str`.

### Secondary goals

6. Sorting, limiting, and skipping on query results.
7. Basic single-field indexes for O(1) point lookups.
8. Lightweight transaction / batch operation.
9. Memory-only storage backend.
10. Better ergonomics with builder-style query construction.

### Non-goals for v2.0

- Multi-process access / file locking coordination.
- Network interface.
- SQL or query-language parsing.
- Joins across collections.

---

## 3. Breaking Changes Summary

| Area | v1 | v2 |
|------|----|----|
| `Document` fields | `{ _id, data, _st }` | `{ id, data }` |
| `find(search: &T)` | Serialized byte-match | `find(predicate: F) where F: Fn(&T) -> bool` |
| `update(search, new_val)` | Replace entire value | `update(predicate, mutator: F) where F: Fn(T) -> T` |
| `delete(search)` | Byte-match | `delete(predicate: F) where F: Fn(&T) -> bool` |
| `new(name: &'static str)` | Static string | `new(config: DbConfig)` or `open(name: impl Into<String>)` |
| `insert_one` / `insert` | Returns Document | Returns Document (same shape, different struct) |
| Binary format | Newline-terminated bincode (broken) | Length-prefixed binary frame |
| Compaction | Always on startup | Threshold-based + manual |

---

## 4. New Data Model

### `Document<T>`

`Status` is removed from the public type. It becomes an internal WAL concept.

```rust
// src/document.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document<T> {
    pub id: Uuid,
    pub data: T,
}

impl<T> Document<T> {
    pub(crate) fn new(data: T) -> Self {
        Document { id: Uuid::new_v4(), data }
    }

    pub(crate) fn with_id(id: Uuid, data: T) -> Self {
        Document { id, data }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Document<U> {
        Document { id: self.id, data: f(self.data) }
    }
}
```

### `WalEntry<T>` (internal)

The append-only log entry that the storage layer writes. Never exposed publicly.

```rust
// src/wal.rs  (internal)

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum WalOp {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct WalEntry {
    pub op:   WalOp,
    pub id:   Uuid,
    pub data: Vec<u8>,   // serialized T; empty for Delete
}
```

### In-memory store

```rust
// src/store.rs  (internal)

use std::collections::HashMap;
use uuid::Uuid;

// Raw bytes per document (avoids re-serializing for persistence writes)
pub(crate) type RawStore = HashMap<Uuid, Vec<u8>>;
```

### `RedDb<SE, ST>`

```rust
// src/lib.rs

pub struct RedDb<SE, ST> {
    config:     DbConfig,
    storage:    ST,
    serializer: SE,
    store:      Arc<RwLock<RawStore>>,
}
```

### `DbConfig`

```rust
// src/config.rs

#[derive(Debug, Clone)]
pub struct DbConfig {
    /// Database file name (without extension).
    pub name: String,

    /// Directory to store the data file. Defaults to current dir.
    pub dir: PathBuf,

    /// Trigger compaction when file_size >= live_data_size * ratio.
    /// Default: 2.0 (compact when file is 2× larger than live data).
    pub compaction_ratio: f64,

    /// If true, load existing data but never write to disk.
    pub read_only: bool,
}

impl DbConfig {
    pub fn new(name: impl Into<String>) -> Self {
        DbConfig {
            name: name.into(),
            dir: PathBuf::from("."),
            compaction_ratio: 2.0,
            read_only: false,
        }
    }

    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = dir.into();
        self
    }

    pub fn compaction_ratio(mut self, ratio: f64) -> Self {
        self.compaction_ratio = ratio;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}
```

---

## 5. Closure-Based Query API

### Core idea

Every query that previously took a search value `&T` now takes a predicate `F: Fn(&T) -> bool`. Every update that previously took a replacement value now takes a mutator `M: Fn(T) -> T`. This means the database **deserializes** each stored document and hands it to the closure — the closure inspects real field values instead of comparing raw bytes.

### `QueryBuilder<T>` — chainable query object

```rust
// src/query.rs

pub struct QueryBuilder<'db, T, SE, ST> {
    db:        &'db RedDb<SE, ST>,
    predicate: Option<Box<dyn Fn(&T) -> bool + Send + Sync>>,
    order_by:  Option<Box<dyn Fn(&T, &T) -> std::cmp::Ordering + Send + Sync>>,
    limit:     Option<usize>,
    skip:      usize,
    _marker:   std::marker::PhantomData<T>,
}

impl<'db, T, SE, ST> QueryBuilder<'db, T, SE, ST>
where
    for<'de> T: Serialize + Deserialize<'de> + Send + Sync + 'static,
    for<'de> SE: Serializer<'de> + Send + Sync,
    ST: Storage + Send + Sync,
{
    pub fn filter<F>(mut self, predicate: F) -> Self
    where F: Fn(&T) -> bool + Send + Sync + 'static
    {
        self.predicate = Some(Box::new(predicate));
        self
    }

    pub fn order_by<F>(mut self, comparator: F) -> Self
    where F: Fn(&T, &T) -> std::cmp::Ordering + Send + Sync + 'static
    {
        self.order_by = Some(Box::new(comparator));
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn skip(mut self, n: usize) -> Self {
        self.skip = n;
        self
    }

    /// Execute query, returning all matching documents.
    pub async fn all(self) -> Result<Vec<Document<T>>> { ... }

    /// Execute query, returning only the first match.
    pub async fn first(self) -> Result<Option<Document<T>>> { ... }

    /// Execute query, returning only IDs.
    pub async fn ids(self) -> Result<Vec<Uuid>> { ... }

    /// Count matching documents without deserializing all fields.
    pub async fn count(self) -> Result<usize> { ... }
}
```

### `UpdateBuilder<T>` — chainable update

```rust
// src/query.rs

pub struct UpdateBuilder<'db, T, SE, ST> {
    db:        &'db RedDb<SE, ST>,
    predicate: Box<dyn Fn(&T) -> bool + Send + Sync>,
    mutator:   Box<dyn Fn(T) -> T + Send + Sync>,
    limit:     Option<usize>,
}

impl<'db, T, SE, ST> UpdateBuilder<'db, T, SE, ST>
where
    for<'de> T: Serialize + Deserialize<'de> + Clone + Send + Sync + 'static,
    for<'de> SE: Serializer<'de> + Send + Sync,
    ST: Storage + Send + Sync,
{
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute update, returning count of modified documents.
    pub async fn exec(self) -> Result<usize> { ... }

    /// Execute update, returning updated documents.
    pub async fn returning(self) -> Result<Vec<Document<T>>> { ... }
}
```

### Top-level `RedDb` methods

```rust
impl<SE, ST> RedDb<SE, ST>
where
    for<'de> SE: Serializer<'de> + Send + Sync,
    ST: Storage + Send + Sync,
{
    // ── construction ──────────────────────────────────────────────────────────

    /// Open (or create) a database with default file storage.
    pub async fn open<T>(config: DbConfig) -> Result<Self>
    where
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync + Debug;

    // ── insert ────────────────────────────────────────────────────────────────

    /// Insert one document; returns the stored Document with its generated ID.
    pub async fn insert_one<T>(&self, value: T) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

    /// Insert many documents in one batch write.
    pub async fn insert<T>(&self, values: Vec<T>) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

    // ── find ──────────────────────────────────────────────────────────────────

    /// Begin building a query. Chain .filter(), .order_by(), .limit(), .skip().
    pub fn query<T>(&self) -> QueryBuilder<T, SE, ST>;

    /// Shorthand: find by exact ID.
    pub async fn get<T>(&self, id: &Uuid) -> Result<Option<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

    /// Shorthand: return all documents.
    pub async fn all<T>(&self) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

    // ── update ────────────────────────────────────────────────────────────────

    /// Update a single document by ID, replacing its data.
    pub async fn update_by_id<T>(&self, id: &Uuid, new_value: T) -> Result<bool>
    where
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

    /// Begin building a closure-based update.
    ///
    /// Example:
    ///   db.update_where(|u: &User| u.active)
    ///     .with(|u| User { score: u.score + 1, ..u })
    ///     .limit(10)
    ///     .exec()
    ///     .await?;
    pub fn update_where<T, F>(&self, predicate: F) -> UpdateWhereBuilder<T, F, SE, ST>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static;

    // ── delete ────────────────────────────────────────────────────────────────

    /// Delete a single document by ID.
    pub async fn delete_by_id<T>(&self, id: &Uuid) -> Result<bool>
    where
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

    /// Delete all documents matching the predicate. Returns count deleted.
    pub async fn delete_where<T, F>(&self, predicate: F) -> Result<usize>
    where
        F: Fn(&T) -> bool + Send + Sync,
        for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

    // ── maintenance ───────────────────────────────────────────────────────────

    /// Force a compaction immediately.
    pub async fn compact(&self) -> Result<()>;

    /// Return live document count.
    pub async fn len(&self) -> usize;

    /// Return true if database contains no documents.
    pub async fn is_empty(&self) -> bool;
}
```

---

## 6. Persistence — Binary File Format

### The v1 problem

All serializers — including bincode — append `\n` (`0x0A`) as a record delimiter. Binary data produced by bincode routinely contains `0x0A` bytes. When the file is read back line-by-line via `BufRead::lines()`, the record is split at the embedded `0x0A`, producing corrupted partial records. This is why the binary backend was never reliable.

### v2 solution: length-prefixed framing

Replace newline delimiters with a 4-byte little-endian length prefix before each record. This works for every serializer, including binary, because the reader always knows exactly how many bytes to consume.

### File layout

```
┌────────────────────────────────────────────────────┐
│  FILE HEADER (32 bytes)                            │
│  [0..8]   magic:   b"REDDB\x00\x02\x00"           │  8 bytes
│  [8..10]  version: u16 LE  (0x0002 = v2)          │  2 bytes
│  [10..11] format:  u8  (0=JSON 1=RON 2=YAML 3=BIN)│  1 byte
│  [11..15] flags:   u32 LE  (reserved, set to 0)   │  4 bytes
│  [15..32] padding: 0x00 × 17                       │ 17 bytes
├────────────────────────────────────────────────────┤
│  RECORD 0                                          │
│  [0..4]   len:  u32 LE  (byte length of payload)  │  4 bytes
│  [4..5]   op:   u8  (0x01=Ins  0x02=Up  0x03=Del) │  1 byte
│  [5..21]  id:   [u8; 16]  (UUID bytes)             │ 16 bytes
│  [21..21+len] payload: serialized T                │ N bytes
├────────────────────────────────────────────────────┤
│  RECORD 1                                          │
│  ...                                               │
└────────────────────────────────────────────────────┘
```

Total per-record overhead: **21 bytes** (4 len + 1 op + 16 uuid).

For Delete records `payload` is empty (`len = 0`) — only the UUID is needed.

### Record reading (load)

```rust
// src/storage/file.rs

async fn read_records(file: &mut File) -> Result<Vec<RawRecord>> {
    // Skip 32-byte header
    file.seek(SeekFrom::Start(32)).await?;

    let mut records = Vec::new();
    let mut buf = [0u8; 21];

    loop {
        match file.read_exact(&mut buf[..4]).await {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let payload_len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;

        file.read_exact(&mut buf[4..21]).await?;

        let op = match buf[4] {
            0x01 => WalOp::Insert,
            0x02 => WalOp::Update,
            0x03 => WalOp::Delete,
            b    => return Err(RedDbError::corrupt(format!("unknown op byte {b:#04x}"))),
        };

        let id = Uuid::from_bytes(buf[5..21].try_into().unwrap());

        let mut payload = vec![0u8; payload_len];
        file.read_exact(&mut payload).await?;

        records.push(RawRecord { op, id, payload });
    }

    Ok(records)
}
```

### Record writing (append)

```rust
async fn write_record(file: &mut File, op: WalOp, id: Uuid, payload: &[u8]) -> Result<()> {
    let len = payload.len() as u32;
    let mut frame = Vec::with_capacity(21 + payload.len());
    frame.extend_from_slice(&len.to_le_bytes());           // 4 bytes
    frame.push(match op {                                  // 1 byte
        WalOp::Insert => 0x01,
        WalOp::Update => 0x02,
        WalOp::Delete => 0x03,
    });
    frame.extend_from_slice(id.as_bytes());                // 16 bytes
    frame.extend_from_slice(payload);                      // N bytes

    file.seek(SeekFrom::End(0)).await?;
    file.write_all(&frame).await?;
    file.sync_data().await?;
    Ok(())
}
```

### File header writing

```rust
fn build_header(format: FormatId) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[0..8].copy_from_slice(b"REDDB\x00\x02\x00");
    h[8..10].copy_from_slice(&2u16.to_le_bytes());
    h[10] = format as u8;
    // bytes 11–31 are zeroed (flags + padding)
    h
}
```

---

## 7. Persistence — WAL & Compaction Strategy

### Append-Only semantics (preserved from v1)

Every write (insert / update / delete) appends a record to the end of the file. The in-memory `RawStore` is the authoritative live view; the file is the recovery journal.

### Load sequence

```
open file
  └─► read header → verify magic + version + format match
  └─► read all records sequentially
        for each record:
          Insert → store raw payload under id
          Update → replace payload under id
          Delete → remove id from map
  └─► check compaction threshold
        if file_byte_size >= live_data_byte_size * config.compaction_ratio
          → compact immediately
  └─► database ready
```

### Compaction

Compaction rewrites the file with exactly one Insert record per live document — no deleted or superseded records survive. This is already done in v1, but v2 adds a threshold check so it does not run on every startup when the file is already clean.

```rust
// src/storage/file.rs

impl<SE: Serializer> FileStorage<SE> {
    pub async fn compact(&self, store: &RawStore) -> Result<()> {
        let tmp_path = self.path.with_extension("tmp");

        {
            let mut tmp = File::create(&tmp_path).await?;
            tmp.write_all(&build_header(self.format_id)).await?;

            for (id, payload) in store.iter() {
                write_record(&mut tmp, WalOp::Insert, *id, payload).await?;
            }
            tmp.sync_all().await?;
        }

        // Atomic rename — on POSIX this is guaranteed to be atomic
        tokio::fs::rename(&tmp_path, &self.path).await?;

        // Reopen for appending
        *self.file.lock().await = open_append(&self.path).await?;
        Ok(())
    }

    fn should_compact(&self, file_size: u64, live_size: u64) -> bool {
        live_size > 0
            && (file_size as f64) >= (live_size as f64) * self.config.compaction_ratio
    }
}
```

### Crash safety

| Scenario | Outcome |
|----------|---------|
| Process killed mid-append | Partial record at EOF — the reader sees `UnexpectedEof` before reading `payload_len` bytes, truncates the partial record, loads everything before it. |
| Process killed mid-compaction | The `.tmp` file is incomplete; `rename` never ran; original file is intact. |
| Disk full during append | `write_all` returns an error; in-memory state already updated; next startup will replay from file (the last append failed so data is not in file, but it is also not in the store on next cold start). For durability-critical use, v2 supports **write-ahead**: write to file first, then update in-memory. |

### Write-ahead option

```rust
pub enum WriteOrder {
    /// Update in-memory first, then persist (default — faster, less durable).
    MemFirst,
    /// Persist first, then update in-memory (durability-first).
    FileFirst,
}
```

Configurable in `DbConfig::write_order`.

---

## 8. Storage Trait Redesign

```rust
// src/storage/mod.rs

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Initialize storage (open or create file, etc.).
    async fn init(config: &DbConfig, format_id: FormatId) -> Result<Self>
    where
        Self: Sized;

    /// Load all WAL entries and return raw byte map.
    async fn load(&self) -> Result<(RawStore, StorageStats)>;

    /// Append a single WAL entry.
    async fn append(&self, op: WalOp, id: Uuid, payload: &[u8]) -> Result<()>;

    /// Append multiple WAL entries atomically (single fsync).
    async fn append_batch(&self, entries: &[(WalOp, Uuid, Vec<u8>)]) -> Result<()>;

    /// Rewrite the file with exactly the provided store contents.
    async fn compact(&self, store: &RawStore) -> Result<()>;

    /// Size of the backing storage in bytes (for compaction ratio check).
    async fn file_size(&self) -> Result<u64>;
}

pub struct StorageStats {
    pub total_records:  usize,   // records read from file
    pub live_records:   usize,   // after replaying deletes
    pub file_size:      u64,
    pub compacted:      bool,    // did we compact on load?
}
```

### `FileStorage<SE>`

```rust
// src/storage/file.rs

pub struct FileStorage<SE> {
    path:        PathBuf,
    config:      DbConfig,
    format_id:   FormatId,
    file:        Arc<Mutex<File>>,
    serializer:  SE,
}
```

### `MemStorage` (new in v2)

A no-persistence backend — the database still lives entirely in RAM (as always), but writes are never journaled to disk. Useful for tests and cases where crash recovery is not needed.

```rust
// src/storage/mem.rs

pub struct MemStorage {
    log: Arc<Mutex<Vec<(WalOp, Uuid, Vec<u8>)>>>,
}

#[async_trait]
impl Storage for MemStorage {
    async fn init(_config: &DbConfig, _format: FormatId) -> Result<Self> {
        Ok(MemStorage { log: Default::default() })
    }

    async fn load(&self) -> Result<(RawStore, StorageStats)> {
        let mut store = RawStore::new();
        for (op, id, data) in self.log.lock().await.iter() {
            match op {
                WalOp::Insert | WalOp::Update => { store.insert(*id, data.clone()); }
                WalOp::Delete => { store.remove(id); }
            }
        }
        let live = store.len();
        let stats = StorageStats {
            total_records: self.log.lock().await.len(),
            live_records: live,
            file_size: 0,
            compacted: false,
        };
        Ok((store, stats))
    }

    async fn append(&self, op: WalOp, id: Uuid, payload: &[u8]) -> Result<()> {
        self.log.lock().await.push((op, id, payload.to_vec()));
        Ok(())
    }

    async fn append_batch(&self, entries: &[(WalOp, Uuid, Vec<u8>)]) -> Result<()> {
        self.log.lock().await.extend_from_slice(entries);
        Ok(())
    }

    async fn compact(&self, store: &RawStore) -> Result<()> {
        let mut log = self.log.lock().await;
        log.clear();
        for (id, data) in store.iter() {
            log.push((WalOp::Insert, *id, data.clone()));
        }
        Ok(())
    }

    async fn file_size(&self) -> Result<u64> { Ok(0) }
}
```

---

## 9. Serializer Redesign

### Trait

The trait signature is simplified. The `format()` method now returns a `FormatId` enum instead of a `Serializers` enum containing a file extension string.

```rust
// src/serializer/mod.rs

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatId {
    Json = 0,
    Ron  = 1,
    Yaml = 2,
    Bin  = 3,
}

impl FormatId {
    pub fn extension(self) -> &'static str {
        match self {
            FormatId::Json => ".json",
            FormatId::Ron  => ".ron",
            FormatId::Yaml => ".yaml",
            FormatId::Bin  => ".bin",
        }
    }
}

pub trait Serializer: Default + Send + Sync {
    fn format_id(&self) -> FormatId;

    fn serialize<T>(&self, val: &T) -> Result<Vec<u8>>
    where
        for<'de> T: Serialize + Deserialize<'de>;

    fn deserialize<T>(&self, bytes: &[u8]) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de>;
}
```

Key differences from v1:
- No lifetime on the trait itself (`Serializer<'a>` → `Serializer`).
- No `\n` appended by serializers — framing is now the file format's job.
- `format_id()` returns a `FormatId` (byte-sized enum) rather than `Serializers(String)`.

### Binary serializer fix

```rust
// src/serializer/bin.rs

#[cfg(feature = "bin_ser")]
impl Serializer for Bin {
    fn format_id(&self) -> FormatId { FormatId::Bin }

    fn serialize<T>(&self, val: &T) -> Result<Vec<u8>>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        // No newline appended — length-prefix framing handles delimiting.
        Ok(bincode::serialize(val)?)
    }

    fn deserialize<T>(&self, bytes: &[u8]) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(bincode::deserialize(bytes)?)
    }
}
```

### JSON serializer fix

```rust
// src/serializer/json.rs

#[cfg(feature = "json_ser")]
impl Serializer for Json {
    fn format_id(&self) -> FormatId { FormatId::Json }

    fn serialize<T>(&self, val: &T) -> Result<Vec<u8>>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(serde_json::to_vec(val)?)  // no trailing \n
    }

    fn deserialize<T>(&self, bytes: &[u8]) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(serde_json::from_slice(bytes)?)
    }
}
```

Remove the debug `println!` from the YAML serializer.

---

## 10. Indexing System

Indexes are optional per-collection. They live entirely in memory and are rebuilt on startup by scanning the store. They are not persisted separately (the WAL is the source of truth).

### Index trait

```rust
// src/index.rs

pub trait Index<T>: Send + Sync {
    /// Called after every insert/update with the new value.
    fn on_insert(&mut self, id: Uuid, value: &T);

    /// Called after every update with the old value removed.
    fn on_remove(&mut self, id: Uuid, value: &T);

    /// Point lookup: returns IDs matching an exact key.
    fn lookup(&self, key: &dyn Any) -> Vec<Uuid>;
}
```

### `HashIndex<T, K>` — exact-match O(1) lookup

```rust
// src/index.rs

pub struct HashIndex<T, K>
where
    K: Eq + Hash + Clone + Send + Sync,
    T: Send + Sync,
{
    extractor: Box<dyn Fn(&T) -> K + Send + Sync>,
    map:       HashMap<K, HashSet<Uuid>>,
}

impl<T, K> HashIndex<T, K>
where
    K: Eq + Hash + Clone + Send + Sync,
    T: Send + Sync,
{
    pub fn new<F>(extractor: F) -> Self
    where
        F: Fn(&T) -> K + Send + Sync + 'static,
    {
        HashIndex { extractor: Box::new(extractor), map: HashMap::new() }
    }

    pub fn get(&self, key: &K) -> &[Uuid] { ... }
}
```

### Using an index

```rust
// User builds the index once and passes it to the collection
let mut db = RedDb::<Json, FileStorage<Json>>::open::<User>(config).await?;

// Register an index on User::email
let idx = HashIndex::new(|u: &User| u.email.clone());
db.add_index("by_email", idx);

// Later: query via index (O(1) lookup)
let results = db.query::<User>()
    .using_index("by_email", &"alice@example.com")
    .all()
    .await?;
```

`add_index` triggers a full scan to populate the index, then the index is kept up to date on every write.

---

## 11. Transaction Support

Transactions batch multiple operations and apply them atomically: either all operations persist and update the in-memory store, or none do.

```rust
// src/transaction.rs

pub struct Transaction<'db, SE, ST> {
    db:  &'db RedDb<SE, ST>,
    ops: Vec<TxOp>,
}

enum TxOp {
    Insert { id: Uuid, payload: Vec<u8> },
    Update { id: Uuid, payload: Vec<u8> },
    Delete { id: Uuid },
}

impl<'db, SE, ST> Transaction<'db, SE, ST>
where
    for<'de> SE: Serializer + Send + Sync,
    ST: Storage + Send + Sync,
{
    /// Stage an insert (does not write to disk yet).
    pub fn insert<T>(&mut self, value: T) -> Result<Uuid>
    where
        for<'de> T: Serialize + Deserialize<'de>;

    /// Stage an update.
    pub fn update<T>(&mut self, id: Uuid, new_value: T) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de>;

    /// Stage a delete.
    pub fn delete(&mut self, id: Uuid);

    /// Commit: write all staged ops as a single batch append, then update
    /// in-memory store. Either everything succeeds or nothing is applied.
    pub async fn commit(self) -> Result<()>;

    /// Discard all staged operations without writing anything.
    pub fn rollback(self) {}
}

// Usage:
impl<SE, ST> RedDb<SE, ST> {
    pub fn begin(&self) -> Transaction<SE, ST> {
        Transaction { db: self, ops: Vec::new() }
    }
}
```

Atomicity guarantee: `append_batch` writes all records and calls a single `fsync`. If the process dies mid-write, the partial tail records will be detected on next startup (incomplete frame → truncated during load).

---

## 12. Complete API Reference

```rust
// ── Open / Close ─────────────────────────────────────────────────────────────

/// Open or create a database. T is the initial collection type (used for load).
pub async fn open<T>(config: DbConfig) -> Result<RedDb<SE, ST>>;

/// Shorthand with default config (current dir, default compaction ratio).
pub async fn with_name<T>(name: impl Into<String>) -> Result<RedDb<SE, ST>>;

// ── Insert ────────────────────────────────────────────────────────────────────

pub async fn insert_one<T>(&self, value: T)      -> Result<Document<T>>;
pub async fn insert<T>(&self, values: Vec<T>)    -> Result<Vec<Document<T>>>;

// ── Point lookup ─────────────────────────────────────────────────────────────

pub async fn get<T>(&self, id: &Uuid)            -> Result<Option<Document<T>>>;
pub async fn all<T>(&self)                       -> Result<Vec<Document<T>>>;
pub async fn len(&self)                          -> usize;
pub async fn is_empty(&self)                     -> bool;

// ── Closure queries ───────────────────────────────────────────────────────────

/// Returns a QueryBuilder for method chaining.
pub fn query<T>(&self)                           -> QueryBuilder<T, SE, ST>;

// QueryBuilder methods:
//   .filter(|t: &T| bool)          → filter by predicate
//   .order_by(|a: &T, b: &T| Ordering) → sort result
//   .limit(n: usize)               → take at most n
//   .skip(n: usize)                → skip first n
//   .all()   -> Result<Vec<Document<T>>>
//   .first() -> Result<Option<Document<T>>>
//   .ids()   -> Result<Vec<Uuid>>
//   .count() -> Result<usize>

// ── Update ────────────────────────────────────────────────────────────────────

/// Replace data of a document identified by ID. Returns true if found.
pub async fn update_by_id<T>(&self, id: &Uuid, new_value: T) -> Result<bool>;

/// Closure-based update builder.
pub fn update_where<T, F>(&self, predicate: F)  -> UpdateWhereBuilder<T, F, SE, ST>
where F: Fn(&T) -> bool + Send + Sync + 'static;

// UpdateWhereBuilder methods:
//   .with(|t: T| T)         → transformation to apply
//   .limit(n: usize)        → stop after n updates
//   .exec()     -> Result<usize>          (count only)
//   .returning() -> Result<Vec<Document<T>>>  (updated docs)

// ── Delete ────────────────────────────────────────────────────────────────────

/// Delete by ID. Returns true if found and deleted.
pub async fn delete_by_id<T>(&self, id: &Uuid)  -> Result<bool>;

/// Delete all matching. Returns count deleted.
pub async fn delete_where<T, F>(&self, predicate: F) -> Result<usize>
where F: Fn(&T) -> bool + Send + Sync,
      for<'de> T: Serialize + Deserialize<'de> + Send + Sync;

// ── Transactions ──────────────────────────────────────────────────────────────

pub fn begin(&self)                              -> Transaction<SE, ST>;

// Transaction methods:
//   .insert(value: T)     -> Result<Uuid>
//   .update(id, value: T) -> Result<()>
//   .delete(id: Uuid)
//   .commit()             -> Result<()>
//   .rollback()

// ── Maintenance ───────────────────────────────────────────────────────────────

/// Force compaction now.
pub async fn compact(&self)                      -> Result<()>;

/// Storage statistics.
pub async fn stats(&self)                        -> Result<StorageStats>;
```

---

## 13. Error Handling

Replace the single `RedDbErrorKind` enum (20+ variants, many unused) with a leaner set grouped by category.

```rust
// src/error.rs

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RedDbError>;

#[derive(Debug, Error)]
pub enum RedDbError {
    // Storage / IO
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("file header corrupt or wrong format")]
    CorruptHeader,

    #[error("record at offset {offset} is truncated")]
    CorruptRecord { offset: u64 },

    #[error("format mismatch: file has {file:?}, config expects {expected:?}")]
    FormatMismatch { file: FormatId, expected: FormatId },

    // Serialization
    #[error("serialization failed: {0}")]
    Serialize(String),

    #[error("deserialization failed: {0}")]
    Deserialize(String),

    // Concurrency
    #[error("lock poisoned")]
    LockPoisoned,

    // Logic
    #[error("document not found: {0}")]
    NotFound(Uuid),

    #[error("database is read-only")]
    ReadOnly,

    #[error("transaction conflict: id {0} was modified concurrently")]
    TransactionConflict(Uuid),
}
```

---

## 14. Configuration

```rust
// src/config.rs

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub name:             String,
    pub dir:              PathBuf,
    pub compaction_ratio: f64,
    pub write_order:      WriteOrder,
    pub read_only:        bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOrder {
    MemFirst,   // default — update memory, then persist
    FileFirst,  // persist first, then update memory
}

impl Default for DbConfig {
    fn default() -> Self {
        DbConfig {
            name:             "reddb".to_string(),
            dir:              PathBuf::from("."),
            compaction_ratio: 2.0,
            write_order:      WriteOrder::MemFirst,
            read_only:        false,
        }
    }
}

impl DbConfig {
    pub fn new(name: impl Into<String>) -> Self {
        DbConfig { name: name.into(), ..Default::default() }
    }
    pub fn dir(mut self, d: impl Into<PathBuf>) -> Self { self.dir = d.into(); self }
    pub fn compaction_ratio(mut self, r: f64) -> Self { self.compaction_ratio = r; self }
    pub fn write_first(mut self) -> Self { self.write_order = WriteOrder::FileFirst; self }
    pub fn read_only(mut self) -> Self { self.read_only = true; self }
}
```

### Type aliases (same as v1, updated)

```rust
#[cfg(feature = "json_ser")]
pub type JsonDb = RedDb<serializer::Json, FileStorage<serializer::Json>>;

#[cfg(feature = "bin_ser")]
pub type BinDb = RedDb<serializer::Bin, FileStorage<serializer::Bin>>;

#[cfg(feature = "ron_ser")]
pub type RonDb = RedDb<serializer::Ron, FileStorage<serializer::Ron>>;

#[cfg(feature = "yaml_ser")]
pub type YamlDb = RedDb<serializer::Yaml, FileStorage<serializer::Yaml>>;

/// In-memory only (any serializer; default JSON).
#[cfg(feature = "json_ser")]
pub type MemJsonDb = RedDb<serializer::Json, MemStorage>;
```

---

## 15. Migration Guide from v1

### 1. Update Cargo.toml

```toml
# Before
reddb = { version = "0.2", features = ["json_ser"] }

# After
reddb = { version = "2.0", features = ["json_ser"] }
```

### 2. Opening the database

```rust
// v1
let db: JsonDb = JsonDb::new::<MyType>("mydb")?;

// v2
let db: JsonDb = JsonDb::open::<MyType>(DbConfig::new("mydb")).await?;
// or shorthand:
let db: JsonDb = JsonDb::with_name::<MyType>("mydb").await?;
```

### 3. Document fields

```rust
// v1
doc._id   →  v2: doc.id
doc.data  →  v2: doc.data   (same)
doc._st   →  v2: removed (internal)
```

### 4. Find queries

```rust
// v1 — exact serialized match
let results = db.find(&User { name: "alice".into(), age: 30 }).await?;

// v2 — closure predicate (can express partial match, range, etc.)
let results = db.query::<User>()
    .filter(|u| u.name == "alice")
    .all()
    .await?;

// v2 equivalent of v1 find_one (by id)
// v1: db.find_one::<User>(&id).await?
// v2: db.get::<User>(&id).await?
```

### 5. Update queries

```rust
// v1 — replace entire document
let n = db.update(&old_user, &new_user).await?;

// v2 — mutator closure
let n = db.update_where(|u: &User| u.name == "alice")
    .with(|u| User { age: u.age + 1, ..u })
    .exec()
    .await?;
```

### 6. Delete queries

```rust
// v1
let n = db.delete(&user).await?;

// v2
let n = db.delete_where(|u: &User| u.name == "alice").await?;
```

### 7. File format

v1 and v2 files are **not compatible**. On first run with v2, the old `.json` / `.bin` etc. file must be migrated or removed. A migration utility will be provided:

```bash
reddb-migrate --from mydb.json --to mydb_v2.json --format json
```

Or in code:

```rust
reddb::migrate::from_v1("mydb.json", DbConfig::new("mydb")).await?;
```

---

## 16. Example Code

### Basic CRUD

```rust
use reddb::{JsonDb, DbConfig, Document};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    name:  String,
    email: String,
    age:   u32,
    score: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db: JsonDb = JsonDb::open::<User>(DbConfig::new("users")).await?;

    // Insert
    let alice = db.insert_one(User {
        name:  "Alice".into(),
        email: "alice@example.com".into(),
        age:   30,
        score: 95.5,
    }).await?;
    println!("inserted: {}", alice.id);

    // Batch insert
    let _others = db.insert(vec![
        User { name: "Bob".into(),   email: "bob@example.com".into(),   age: 25, score: 87.0 },
        User { name: "Carol".into(), email: "carol@example.com".into(), age: 35, score: 72.3 },
    ]).await?;

    // Find by ID
    let found: Option<Document<User>> = db.get(&alice.id).await?;
    assert_eq!(found.unwrap().data.name, "Alice");

    // Find all adults sorted by score descending
    let top_scorers: Vec<Document<User>> = db.query::<User>()
        .filter(|u| u.age >= 18)
        .order_by(|a, b| b.score.partial_cmp(&a.score).unwrap())
        .limit(10)
        .all()
        .await?;
    println!("top scorer: {}", top_scorers[0].data.name);

    // Update — increment age for all users named Alice
    let updated = db.update_where(|u: &User| u.name == "Alice")
        .with(|u| User { age: u.age + 1, ..u })
        .exec()
        .await?;
    println!("updated {} user(s)", updated);

    // Delete users with score below threshold
    let removed = db.delete_where(|u: &User| u.score < 75.0).await?;
    println!("pruned {} low-score user(s)", removed);

    // Stats
    println!("live documents: {}", db.len().await);
    Ok(())
}
```

### Transactions

```rust
let mut tx = db.begin();
let id1 = tx.insert(User { name: "Dave".into(), email: "dave@x.com".into(), age: 22, score: 88.0 })?;
let id2 = tx.insert(User { name: "Eve".into(),  email: "eve@x.com".into(),  age: 28, score: 91.0 })?;
tx.delete(alice.id);
tx.commit().await?;    // single fsync, all-or-nothing
```

### No-persistence database (tests / ephemeral use)

```rust
#[cfg(feature = "json_ser")]
use reddb::MemJsonDb;

// Data lives in RAM as always; nothing is written to disk
let db: MemJsonDb = MemJsonDb::open::<User>(DbConfig::new("test")).await?;

let doc = db.insert_one(User { ... }).await?;
let result = db.query::<User>().filter(|u| u.age > 18).first().await?;
assert!(result.is_some());
```

### Binary database

```rust
use reddb::{BinDb, DbConfig};

// BinDb uses the length-prefix binary format — no line-delimiter bug
let db: BinDb = BinDb::open::<User>(DbConfig::new("users_bin")).await?;

let doc = db.insert_one(User { name: "Alice".into(), ..default_user() }).await?;

// Works correctly with binary data in fields too
#[derive(Serialize, Deserialize, Debug, Clone)]
struct BlobDoc {
    name:    String,
    payload: Vec<u8>,   // arbitrary bytes — safe with v2
}

let db: BinDb = BinDb::open::<BlobDoc>(DbConfig::new("blobs")).await?;
let _ = db.insert_one(BlobDoc {
    name:    "image".into(),
    payload: vec![0x00, 0x0A, 0xFF, 0x0A, 0x42],   // 0x0A would break v1
}).await?;
```

### Using the index

```rust
use reddb::index::HashIndex;

let db: JsonDb = JsonDb::open::<User>(DbConfig::new("users")).await?;
db.add_index("email", HashIndex::new(|u: &User| u.email.clone()));

// O(1) lookup (no full scan)
let results = db.query::<User>()
    .using_index("email", &"alice@example.com".to_string())
    .all()
    .await?;
```

---

## 17. Cargo.toml Changes

```toml
[package]
name        = "reddb"
version     = "2.0.0"
edition     = "2021"
authors     = ["Pablo Magaz"]
description = "Minimalistic embedded async document database with closure-based queries and binary persistence"
license     = "MIT OR Apache-2.0"
keywords    = ["database", "embedded", "nosql", "async", "serde"]
categories  = ["database", "data-structures"]

[features]
default  = []
json_ser = ["serde_json"]
ron_ser  = ["ron"]
yaml_ser = ["serde_yaml"]
bin_ser  = ["bincode"]
# Enable all for tests / benchmarks
full     = ["json_ser", "ron_ser", "yaml_ser", "bin_ser"]

[dependencies]
uuid        = { version = "1",   features = ["serde", "v4"] }  # upgrade from 0.8
anyhow      = "1"
thiserror   = "1"
tokio       = { version = "1", features = ["macros", "fs", "sync", "rt-multi-thread"] }
serde       = { version = "1", features = ["derive"] }
futures     = "0.3"
async-trait = "0.1"
# base64 removed — was not used in any actual logic

# optional serializers
serde_json  = { version = "1",   optional = true }
ron         = { version = "0.8", optional = true }
serde_yaml  = { version = "0.9", optional = true }
bincode     = { version = "1",   optional = true }

[dev-dependencies]
tokio-test  = "0.4"
tempfile    = "3"    # for creating temp dirs in integration tests
```

Key upgrades:
- `uuid` 0.8 → 1.x
- `tokio` 0.2 → 1.x (major version, required)
- `base64` removed (unused in v1)
- `tempfile` added for tests

---

## 18. Implementation Roadmap

### Phase 1 — Foundation (core correctness)

| Task | Files | Notes |
|------|-------|-------|
| Upgrade `uuid` and `tokio` to current major versions | `Cargo.toml`, all files | Required before anything else |
| Remove `Status` from `Document`; add `WalEntry` | `document.rs`, `wal.rs` | Breaking change |
| Implement length-prefix file format | `storage/file.rs` | Fixes binary backend |
| Fix binary serializer (remove `\n`) | `serializer/bin.rs` | Depends on above |
| Fix all serializers (remove `\n`) | `serializer/{json,ron,yaml}.rs` | Cleanup |
| Remove debug `println!` from YAML serializer | `serializer/yaml.rs` | Trivial |
| New `DbConfig` struct, replace `&'static str` | `config.rs`, `lib.rs` | API break |
| Implement `MemStorage` | `storage/mem.rs` | Needed for tests |

### Phase 2 — Query API

| Task | Files | Notes |
|------|-------|-------|
| Implement `QueryBuilder` with `filter`, `limit`, `skip`, `order_by` | `query.rs` | Core feature |
| Implement `UpdateWhereBuilder` with mutator closure | `query.rs` | Core feature |
| Implement `delete_where` | `lib.rs` | Core feature |
| Implement `get` (by id, returns `Option`) | `lib.rs` | Replace `find_one` |
| Remove old `find`, `find_one`, `find_all`, `find_uuids` | `lib.rs` | Deprecation |

### Phase 3 — Persistence improvements

| Task | Files | Notes |
|------|-------|-------|
| Threshold-based compaction check on startup | `storage/file.rs` | Skip compaction when not needed |
| Manual `compact()` on `RedDb` | `lib.rs` | Expose to users |
| `WriteOrder::FileFirst` option | `storage/file.rs` | Durability mode |
| `StorageStats` struct and `stats()` method | `lib.rs`, `storage/` | Observability |
| `migrate::from_v1()` utility | `migrate.rs` | Migration path |

### Phase 4 — Advanced features

| Task | Files | Notes |
|------|-------|-------|
| `Transaction` / `begin` / `commit` / `rollback` | `transaction.rs` | Batch atomicity |
| `HashIndex` + `add_index` / `using_index` | `index.rs` | O(1) point lookup |
| Benchmarks (criterion) | `benches/` | Compare v1 vs v2 |
| Full test suite with `MemStorage` | `tests/` | No disk I/O in unit tests |

---

*End of RedDB v2.0 proposal.*
