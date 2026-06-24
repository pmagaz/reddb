use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use reddb::MemDb;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Record {
    id: u32,
    value: String,
}

fn record(n: u32) -> Record {
    Record {
        id: n,
        value: format!("value_{n}"),
    }
}

// ── insert ────────────────────────────────────────────────────────────────────

fn bench_insert_one(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("insert_one", |b| {
        b.to_async(&rt).iter(|| async {
            let db = MemDb::new::<Record>("_").await.unwrap();
            db.insert_one(record(1)).await.unwrap();
        });
    });
}

fn bench_insert_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("insert_batch");
    for size in [10u32, 100, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let db = MemDb::new::<Record>("_").await.unwrap();
                let records: Vec<Record> = (0..size).map(record).collect();
                db.insert(records).await.unwrap();
            });
        });
    }
    group.finish();
}

// ── query ─────────────────────────────────────────────────────────────────────

async fn seeded(n: u32) -> MemDb {
    let db = MemDb::new::<Record>("_").await.unwrap();
    let records: Vec<Record> = (0..n).map(record).collect();
    db.insert(records).await.unwrap();
    db
}

fn bench_find_all(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("find_all");
    for size in [100u32, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let db = rt.block_on(seeded(size));
            b.to_async(&rt)
                .iter(|| async { db.find_all::<Record>().await.unwrap() });
        });
    }
    group.finish();
}

fn bench_query_filter(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("query_filter");
    for size in [100u32, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let db = rt.block_on(seeded(size));
            b.to_async(&rt).iter(|| async {
                db.query::<Record>()
                    .filter(|r| r.id % 2 == 0)
                    .all()
                    .await
                    .unwrap()
            });
        });
    }
    group.finish();
}

// ── update ────────────────────────────────────────────────────────────────────

fn bench_update_one(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("update_one", |b| {
        let db = rt.block_on(seeded(100));
        let ids: Vec<_> = rt
            .block_on(db.find_all::<Record>())
            .unwrap()
            .into_iter()
            .map(|d| d.id)
            .collect();
        let target = ids[0];
        b.to_async(&rt)
            .iter(|| async { db.update_one(&target, record(999)).await.unwrap() });
    });
}

// ── delete ────────────────────────────────────────────────────────────────────

fn bench_delete_one(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("delete_one", |b| {
        b.to_async(&rt).iter(|| async {
            let db = MemDb::new::<Record>("_").await.unwrap();
            let doc = db.insert_one(record(1)).await.unwrap();
            db.delete_one::<Record>(&doc.id).await.unwrap()
        });
    });
}

// ── hash index ────────────────────────────────────────────────────────────────

fn bench_index_lookup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("index_lookup");
    for size in [100u32, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let db = rt.block_on(async {
                let db = seeded(size).await;
                db.add_index::<Record, _>("by_id", |r| r.id.to_string())
                    .await
                    .unwrap();
                db
            });
            b.to_async(&rt)
                .iter(|| async { db.using_index::<Record>("by_id", "50").await.unwrap() });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_insert_one,
    bench_insert_batch,
    bench_find_all,
    bench_query_filter,
    bench_update_one,
    bench_delete_one,
    bench_index_lookup,
);
criterion_main!(benches);
