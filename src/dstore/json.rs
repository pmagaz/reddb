use serde::{Deserialize, Serialize};
use serde_json::Error;
use serde_json::Result as JsonResult;

pub type SerializeError = Error;
pub type DeserializeError = ();

pub fn serialize<T>(value: &T) -> JsonResult<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec(value)
}
