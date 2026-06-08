# RedDB v2.0 — Implementation Progress

## How this works
- One feature at a time, in phase order.
- Every feature ships with its own unit tests **and** integration tests.
- A git commit is made for **each feature** once all its tests pass.
- Status: ✅ Done | 🔄 In progress | ⬜ Pending

---

## Phase 1 — Foundation

| # | Task | Tests | Status | Commit |
|---|------|-------|--------|--------|
| 1.1 | Upgrade `uuid` 0.8 → 1.x and `tokio` 0.2 → 1.x | — | ✅ | `fbe8cfd` |
| 1.2 | New `RedDbError` enum (lean, typed) | unit | ✅ | `0f4a1b0` |
| 1.3 | `DbConfig` struct replacing `&'static str` | unit | ✅ | `9d34680` |
| 1.4 | Clean `Document<T>` — remove `Status`, rename `_id` → `id` | unit | ✅ | `eb686ea` |
| 1.5 | Internal `WalOp` + `WalEntry` types | unit | ✅ | `44c56bb` |
| 1.6+1.7 | Redesign `Serializer` trait (FormatId, no lifetime) + fix all serializers | unit | ✅ | `df57661` |
| 1.8 | Length-prefix binary file format in `FileStorage` | unit + integration | ✅ | `b6ef3bb` |
| 1.9 | `MemStorage` backend | unit + integration | ✅ | `88e3d59` |
| 1.10 | Async constructors, remove thread::spawn, dead code cleanup | unit + integration | ✅ | `e599487` |

---

## Phase 2 — Closure-Based Query API

| # | Task | Tests | Status | Commit |
|---|------|-------|--------|--------|
| 2.1+2.2 | `QueryBuilder` — `.filter()`, `.order_by()`, `.skip()`, `.limit()`, `.all()`, `.first()`, `.count()`, `.ids()` | unit + integration | ✅ | `40ac0d5` |
| 2.3 | `UpdateWhereBuilder` — `.limit()`, `.exec()`, `.returning()` | unit + integration | ✅ | `3363dee` |
| 2.4 | `delete_where(predicate)` | unit + integration | ✅ | `8ca3435` |

---

## Phase 3 — Persistence Improvements

| # | Task | Tests | Status | Commit |
|---|------|-------|--------|--------|
| 3.1 | Threshold-based compaction on startup | unit + integration | ✅ | `TBD` |
| 3.2 | Manual `compact()` on `RedDb` | unit + integration | ✅ | `TBD` |
| 3.3 | `WriteOrder::FileFirst` option | unit + integration | ✅ | `TBD` |
| 3.4 | `StorageStats` struct + `stats()` method | unit + integration | ✅ | `TBD` |

---

## Phase 4 — Advanced Features

| # | Task | Tests | Status | Commit |
|---|------|-------|--------|--------|
| 4.1 | `Transaction` — `begin()`, `commit()`, `rollback()` | unit + integration | ⬜ | — |
| 4.2 | `HashIndex` — `add_index()`, `using_index()` | unit + integration | ⬜ | — |
| 4.3 | Benchmarks (criterion) | — | ⬜ | — |
