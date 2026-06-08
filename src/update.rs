use std::fmt::Debug;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::config::WriteOrder;
use crate::document::Document;
use crate::error::Result;
use crate::serializer::Serializer;
use crate::storage::Storage;
use crate::wal::WalOp;
use crate::RedDb;

/// Builder for closure-based bulk updates, returned by [`RedDb::update_where`].
///
/// Select documents with `.filter` (set at construction), cap with `.limit()`,
/// provide the transformation with `.with()`, then execute via `.exec()` or
/// `.returning()`.
pub struct UpdateWhereBuilder<'db, T, F, SE, ST>
where
    F: Fn(&T) -> bool + Send + Sync + 'static,
{
    db: &'db RedDb<SE, ST>,
    predicate: F,
    limit: Option<usize>,
    _marker: PhantomData<T>,
}

impl<'db, T, F, SE, ST> UpdateWhereBuilder<'db, T, F, SE, ST>
where
    F: Fn(&T) -> bool + Send + Sync + 'static,
    SE: Serializer + Debug,
    for<'de> ST: Storage + Debug + Send + Sync + 'static,
    for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
{
    pub(crate) fn new(db: &'db RedDb<SE, ST>, predicate: F) -> Self {
        Self { db, predicate, limit: None, _marker: PhantomData }
    }

    /// Stop after updating at most `n` documents.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the update using `transform` and return the count of updated documents.
    pub async fn exec<G>(self, transform: G) -> Result<usize>
    where
        G: Fn(T) -> T + Send + Sync,
    {
        Ok(self.run(transform).await?.len())
    }

    /// Execute the update using `transform` and return the updated documents.
    pub async fn returning<G>(self, transform: G) -> Result<Vec<Document<T>>>
    where
        G: Fn(T) -> T + Send + Sync,
    {
        self.run(transform).await
    }

    async fn run<G>(self, transform: G) -> Result<Vec<Document<T>>>
    where
        G: Fn(T) -> T + Send + Sync,
    {
        if self.db.write_order == WriteOrder::FileFirst {
            // Collect matches and build transformed documents under read lock.
            let updates: Vec<(crate::Uuid, Vec<u8>, T)> = {
                let data = self.db.read_lock().await?;
                let mut updates = Vec::new();
                for (id, raw) in data.iter() {
                    if let Some(ref lim) = self.limit {
                        if updates.len() >= *lim {
                            break;
                        }
                    }
                    let value: T = self.db.deserialize_raw(raw)?;
                    if (self.predicate)(&value) {
                        let new_value = transform(value);
                        let new_raw = self.db.serialize_raw(&new_value)?;
                        updates.push((*id, new_raw, new_value));
                    }
                }
                updates
            };

            let docs: Vec<Document<T>> = updates.iter()
                .map(|(id, _, v)| Document::new(*id, v.clone()))
                .collect();

            if !docs.is_empty() {
                self.db.storage_persist(&docs, WalOp::Update).await?;
                let mut data = self.db.write_lock().await?;
                for (id, raw, _) in updates {
                    if let Some(entry) = data.get_mut(&id) {
                        *entry = raw;
                    }
                }
            }

            Ok(docs)
        } else {
            // MemoryFirst: update in-memory first under write lock, then persist.
            let updated: Vec<Document<T>> = {
                let mut data = self.db.write_lock().await?;

                let mut updated = Vec::new();
                for (id, raw) in data.iter_mut() {
                    if let Some(ref lim) = self.limit {
                        if updated.len() >= *lim {
                            break;
                        }
                    }
                    let value: T = self.db.deserialize_raw(raw)?;
                    if (self.predicate)(&value) {
                        let new_value = transform(value);
                        *raw = self.db.serialize_raw(&new_value)?;
                        updated.push(Document::new(*id, new_value));
                    }
                }
                updated
            };

            if !updated.is_empty() {
                self.db.storage_persist(&updated, WalOp::Update).await?;
            }

            Ok(updated)
        }
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
        ])
        .await
        .unwrap();
        db
    }

    #[tokio::test]
    async fn exec_updates_matching_docs() {
        let db = seeded_db().await;
        let count = db
            .update_where::<Item, _>(|i| i.score >= 20)
            .exec(|mut i| { i.score += 5; i })
            .await
            .unwrap();
        assert_eq!(count, 2);

        let all = db.query::<Item>().order_by(|a, b| a.score.cmp(&b.score)).all().await.unwrap();
        assert_eq!(all[0].data.score, 10); // alpha unchanged
        assert_eq!(all[1].data.score, 25); // beta: 20+5
        assert_eq!(all[2].data.score, 35); // gamma: 30+5
    }

    #[tokio::test]
    async fn exec_returns_count_zero_when_no_match() {
        let db = seeded_db().await;
        let count = db
            .update_where::<Item, _>(|i| i.score > 999)
            .exec(|i| i)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn returning_gives_updated_docs() {
        let db = seeded_db().await;
        let docs = db
            .update_where::<Item, _>(|i| i.name == "alpha")
            .returning(|mut i| { i.name = "ALPHA".into(); i })
            .await
            .unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].data.name, "ALPHA");

        let found = db.find_one::<Item>(&docs[0].id).await.unwrap();
        assert_eq!(found.data.name, "ALPHA");
    }

    #[tokio::test]
    async fn limit_caps_number_of_updates() {
        let db = seeded_db().await;
        let count = db
            .update_where::<Item, _>(|i| i.score >= 10)
            .limit(2)
            .exec(|mut i| { i.score = 0; i })
            .await
            .unwrap();
        assert_eq!(count, 2);

        let zeroed = db.query::<Item>().filter(|i| i.score == 0).count().await.unwrap();
        assert_eq!(zeroed, 2);
    }
}
