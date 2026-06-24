# Changelog

## v2.0.0 (2026-06-24)

Complete rewrite. All phases shipped with unit and integration tests.

### Breaking changes

| Area | v1 | v2 |
|------|----|----|
| `Document` fields | `{ _id, data, _st }` | `{ id, data }` |
| Constructor | `JsonDb::new("name")?` (sync) | `JsonDb::new::<T>("name").await?` (async) |
| `db_name` type | `&'static str` | `&str` via `DbConfig` builder |
| `find(search: &T)` | Exact byte-match on serialized value | Returns `Vec<Document<T>>` by equality |
| `update(search, new_val)` | Replace by serialized byte-match | Replace by equality match |
| `delete(search)` | Delete by serialized byte-match | Delete by equality match |
| Binary file format | Newline-delimited (broken for binary data) | Length-prefix framing (correct for all formats) |
| Startup compaction | Always on every open | Threshold-based (ratio configurable) |
| `Status` in `Document` | Public `_st` field | Removed; internal WAL concept only |

Files written by v1 are **not compatible** with v2.

### New features

**Async constructors**
- `RedDb::new::<T>(name)` and `RedDb::open::<T>(config)` are now `async`
- `MemDb` type alias for a no-persistence, in-memory-only database

**Closure-based query API (`QueryBuilder`)**
- `db.query::<T>().filter(|t| ...).order_by(...).skip(n).limit(n).all().await?`
- Terminal methods: `.all()`, `.first()`, `.count()`, `.ids()`

**Closure-based bulk update (`UpdateWhereBuilder`)**
- `db.update_where::<T, _>(predicate).exec(transform).await?`
- `.returning(transform)` returns the modified documents
- `.limit(n)` caps affected documents

**Closure-based bulk delete**
- `db.delete_where::<T, _>(predicate).await?`

**Transactions**
- `db.begin()` returns a `Transaction` that buffers inserts, updates, and deletes
- `tx.commit().await?` applies all ops atomically (single WAL write)
- `tx.rollback()` silently discards all staged ops

**Hash indexes**
- `db.add_index::<T, _>("name", |t| key).await?` — builds and maintains a string-keyed index
- `db.using_index::<T>("name", "key").await?` — O(1) point lookup
- Indexes are kept current on every insert, update, and delete

**Improved persistence**
- New 32-byte file header: magic, version, format ID
- Length-prefix framing (4 bytes) replaces newline delimiters — fixes binary format corruption
- Threshold-based compaction: compact only when `file_size >= live_size × ratio`
- `db.compact().await?` — manual compaction
- `db.stats().await?` — returns `StorageStats { file_size_bytes, live_document_count, compaction_ratio }`
- `WriteOrder::FileFirst` — write to WAL before updating memory for stronger durability

**Configuration builder (`DbConfig`)**
- `DbConfig::new("name").dir("/path").compaction_ratio(3.0).write_order(WriteOrder::FileFirst)`

**`MemStorage` backend**
- In-memory only; no disk writes; useful for tests and ephemeral use

**AtomicBool index fast-path**
- Write paths skip the index `RwLock` entirely when no indexes are registered

**Benchmarks**
- Criterion benchmarks: `insert_one`, `insert_batch`, `find_all`, `query_filter`, `update_one`, `delete_one`, `index_lookup`
- Run with: `cargo bench --features bin_ser`

### Internal improvements

- `Serializer` trait redesigned: no lifetime parameter, returns `FormatId` enum
- Serializer `mod` declarations gated behind feature flags (no dead-code warnings)
- `Storage` trait made `pub(crate)` (sealed; `WalOp` no longer leaks into public API)
- FileFirst TOCTOU race closed: write lock held during WAL persist for `update_one`, `delete_one`, `update`, `delete`, `delete_where`, `update_where`
- Cross-serializer integration tests: all four file formats (Bin, Json, Ron, Yaml) tested with insert/update/delete/compaction round-trips

---

## v0.2.3 (2021-03-01)

- Hotfix: incorrect Document identifier.
- Add changelog.
- Update Readme.
