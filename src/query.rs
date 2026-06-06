use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::error::Result;
use crate::serializer::Serializer;
use crate::storage::Storage;
use crate::RedDb;
use uuid::Uuid;

/// Lazy, chainable query builder returned by [`RedDb::query`].
///
/// Build the query with `.filter()`, `.order_by()`, `.skip()`, `.limit()`,
/// then execute with `.all()`, `.first()`, `.count()`, or `.ids()`.
pub struct QueryBuilder<'db, T, SE, ST> {
    db: &'db RedDb<SE, ST>,
    filter: Option<Box<dyn Fn(&T) -> bool + Send + Sync + 'static>>,
    order: Option<Box<dyn Fn(&T, &T) -> Ordering + Send + Sync + 'static>>,
    limit: Option<usize>,
    skip: usize,
    _marker: PhantomData<T>,
}

impl<'db, T, SE, ST> QueryBuilder<'db, T, SE, ST>
where
    SE: Serializer + Debug,
    for<'de> ST: Storage + Debug + Send + Sync + 'static,
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
{
    pub(crate) fn new(db: &'db RedDb<SE, ST>) -> Self {
        Self {
            db,
            filter: None,
            order: None,
            limit: None,
            skip: 0,
            _marker: PhantomData,
        }
    }

    /// Keep only documents whose data satisfies `predicate`.
    pub fn filter<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Box::new(predicate));
        self
    }

    /// Sort results using the provided comparator (applied before limit/skip).
    pub fn order_by<F>(mut self, cmp: F) -> Self
    where
        F: Fn(&T, &T) -> Ordering + Send + Sync + 'static,
    {
        self.order = Some(Box::new(cmp));
        self
    }

    /// Return at most `n` documents.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Skip the first `n` matching documents.
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = n;
        self
    }

    async fn execute(self) -> Result<Vec<Document<T>>> {
        // Deserialize all entries while holding the read lock, then release it.
        let mut docs: Vec<Document<T>> = {
            let data = self.db.read_lock().await?;
            data.iter()
                .map(|(id, raw)| {
                    self.db
                        .deserialize_raw::<T>(raw)
                        .map(|value| Document::new(*id, value))
                })
                .collect::<Result<Vec<_>>>()?
        };

        // Filter
        if let Some(ref f) = self.filter {
            docs.retain(|doc| f(&doc.data));
        }

        // Sort
        if let Some(ref cmp) = self.order {
            docs.sort_by(|a, b| cmp(&a.data, &b.data));
        }

        // Skip + limit
        let iter = docs.into_iter().skip(self.skip);
        Ok(match self.limit {
            Some(n) => iter.take(n).collect(),
            None => iter.collect(),
        })
    }

    /// Return all matching documents.
    pub async fn all(self) -> Result<Vec<Document<T>>> {
        self.execute().await
    }

    /// Return the first matching document, or `None` if there are no matches.
    pub async fn first(self) -> Result<Option<Document<T>>> {
        Ok(self.limit(1).execute().await?.into_iter().next())
    }

    /// Return the count of matching documents.
    pub async fn count(self) -> Result<usize> {
        Ok(self.execute().await?.len())
    }

    /// Return only the UUIDs of matching documents.
    pub async fn ids(self) -> Result<Vec<Uuid>> {
        Ok(self.execute().await?.into_iter().map(|d| d.id).collect())
    }
}

#[cfg(test)]
#[cfg(feature = "bin_ser")]
mod tests {
    use super::*;
    use crate::MemDb;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Item {
        name: String,
        score: u32,
    }

    fn item(name: &str, score: u32) -> Item {
        Item { name: name.into(), score }
    }

    async fn seeded_db() -> MemDb {
        let db = MemDb::new::<Item>("_").await.unwrap();
        db.insert(vec![
            item("alpha", 10),
            item("beta",  20),
            item("gamma", 30),
            item("delta", 20),
        ])
        .await
        .unwrap();
        db
    }

    #[tokio::test]
    async fn all_without_filter_returns_all() {
        let db = seeded_db().await;
        let results = db.query::<Item>().all().await.unwrap();
        assert_eq!(results.len(), 4);
    }

    #[tokio::test]
    async fn filter_returns_matching_docs() {
        let db = seeded_db().await;
        let results = db.query::<Item>().filter(|i| i.score == 20).all().await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|d| d.data.score == 20));
    }

    #[tokio::test]
    async fn filter_excludes_non_matching() {
        let db = seeded_db().await;
        let results = db.query::<Item>().filter(|i| i.score > 100).all().await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn limit_caps_results() {
        let db = seeded_db().await;
        let results = db.query::<Item>().limit(2).all().await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn skip_offsets_results() {
        let db = seeded_db().await;
        let all = db.query::<Item>().all().await.unwrap();
        let skipped = db.query::<Item>().skip(2).all().await.unwrap();
        assert_eq!(skipped.len(), all.len() - 2);
    }

    #[tokio::test]
    async fn order_by_sorts_ascending() {
        let db = seeded_db().await;
        let results = db
            .query::<Item>()
            .order_by(|a, b| a.score.cmp(&b.score))
            .all()
            .await
            .unwrap();
        let scores: Vec<u32> = results.iter().map(|d| d.data.score).collect();
        assert_eq!(scores, vec![10, 20, 20, 30]);
    }

    #[tokio::test]
    async fn order_by_sorts_descending() {
        let db = seeded_db().await;
        let results = db
            .query::<Item>()
            .order_by(|a, b| b.score.cmp(&a.score))
            .all()
            .await
            .unwrap();
        let scores: Vec<u32> = results.iter().map(|d| d.data.score).collect();
        assert_eq!(scores, vec![30, 20, 20, 10]);
    }

    #[tokio::test]
    async fn filter_order_limit_skip_combined() {
        let db = seeded_db().await;
        // score >= 20, sorted asc, skip 1, limit 1 → should be the second score-20 item
        let results = db
            .query::<Item>()
            .filter(|i| i.score >= 20)
            .order_by(|a, b| a.score.cmp(&b.score))
            .skip(1)
            .limit(1)
            .all()
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].data.score, 20);
    }

    #[tokio::test]
    async fn first_returns_single_doc() {
        let db = seeded_db().await;
        let result = db
            .query::<Item>()
            .filter(|i| i.score == 30)
            .first()
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data.score, 30);
    }

    #[tokio::test]
    async fn first_returns_none_when_no_match() {
        let db = seeded_db().await;
        let result = db
            .query::<Item>()
            .filter(|i| i.score > 999)
            .first()
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn count_returns_correct_number() {
        let db = seeded_db().await;
        let n = db.query::<Item>().filter(|i| i.score >= 20).count().await.unwrap();
        assert_eq!(n, 3);
    }

    #[tokio::test]
    async fn ids_returns_uuids_only() {
        let db = seeded_db().await;
        let ids = db.query::<Item>().filter(|i| i.score == 10).ids().await.unwrap();
        assert_eq!(ids.len(), 1);
        // Verify UUID is valid (not zeroed)
        assert_ne!(ids[0], Uuid::nil());
    }
}
