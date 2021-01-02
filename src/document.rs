use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::status::Status;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Document<T> {
    pub uuid: Uuid,
    //#[serde(flatten)]
    pub data: T,
    pub _st: Status,
}

impl<'a, T> Document<T>
where
    T: Serialize + Deserialize<'a> + Debug,
{
    pub fn new(id: Uuid, data: T, st: Status) -> Self {
        Self {
            uuid: id,
            data,
            _st: st,
        }
    }
}
