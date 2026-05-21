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
| 1.1 | Upgrade `uuid` 0.8 → 1.x and `tokio` 0.2 → 1.x | — | ⬜ | — |
| 1.2 | New `RedDbError` enum (lean, typed) | unit | ⬜ | — |
| 1.3 | `DbConfig` struct replacing `&'static str` | unit | ⬜ | — |
| 1.4 | Clean `Document<T>` — remove `Status`, rename `_id` → `id` | unit | ⬜ | — |
| 1.5 | Internal `WalOp` + `WalEntry` types | unit | ⬜ | — |
| 1.6 | Redesign `Serializer` trait — no lifetime, no `\n`, `FormatId` | unit | ⬜ | — |
| 1.7 | Fix all serializers (json, ron, yaml, bin) | unit | ⬜ | — |
| 1.8 | Length-prefix binary file format in `FileStorage` | unit + integration | ⬜ | — |
| 1.9 | `MemStorage` backend | unit + integration | ⬜ | — |
| 1.10 | Rebuild `RedDb::open()` on new foundation | unit + integration | ⬜ | — |

---

## Phase 2 — Closure-Based Query API

| # | Task | Tests | Status | Commit |
|---|------|-------|--------|--------|
| 2.1 | `QueryBuilder` — `.filter()`, `.limit()`, `.skip()`, `.order_by()` | unit + integration | ⬜ | — |
| 2.2 | `.all()`, `.first()`, `.count()`, `.ids()` terminals | unit + integration | ⬜ | — |
| 2.3 | `UpdateWhereBuilder` — `.with()` mutator, `.exec()`, `.returning()` | unit + integration | ⬜ | — |
| 2.4 | `delete_where(predicate)` | unit + integration | ⬜ | — |
| 2.5 | `get(id)` → `Option<Document<T>>` | unit + integration | ⬜ | — |
| 2.6 | `all()` shorthand | unit + integration | ⬜ | — |

---

## Phase 3 — Persistence Improvements

| # | Task | Tests | Status | Commit |
|---|------|-------|--------|--------|
| 3.1 | Threshold-based compaction on startup | unit + integration | ⬜ | — |
| 3.2 | Manual `compact()` on `RedDb` | unit + integration | ⬜ | — |
| 3.3 | `WriteOrder::FileFirst` option | unit + integration | ⬜ | — |
| 3.4 | `StorageStats` struct + `stats()` method | unit + integration | ⬜ | — |

---

## Phase 4 — Advanced Features

| # | Task | Tests | Status | Commit |
|---|------|-------|--------|--------|
| 4.1 | `Transaction` — `begin()`, `commit()`, `rollback()` | unit + integration | ⬜ | — |
| 4.2 | `HashIndex` — `add_index()`, `using_index()` | unit + integration | ⬜ | — |
| 4.3 | Benchmarks (criterion) | — | ⬜ | — |

---

## Completed commits

_(none yet)_
