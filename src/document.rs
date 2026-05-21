use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

/// A stored document — wraps user data with a generated unique ID.
/// Operation metadata (insert/update/delete) is kept internal to the
/// storage layer and never exposed here.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Document<T> {
    pub id: Uuid,
    pub data: T,
}

impl<T> Document<T>
where
    T: Debug,
{
    pub fn new(id: Uuid, data: T) -> Self {
        Self { id, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Payload {
        val: u32,
    }

    #[test]
    fn new_stores_id_and_data() {
        let id = Uuid::new_v4();
        let doc = Document::new(id, Payload { val: 42 });
        assert_eq!(doc.id, id);
        assert_eq!(doc.data.val, 42);
    }

    #[test]
    fn clone_is_equal() {
        let doc = Document::new(Uuid::new_v4(), Payload { val: 7 });
        assert_eq!(doc.clone(), doc);
    }

    #[test]
    fn different_ids_are_not_equal() {
        let data = Payload { val: 1 };
        let a = Document::new(Uuid::new_v4(), data.clone());
        let b = Document::new(Uuid::new_v4(), data);
        assert_ne!(a, b);
    }
}
