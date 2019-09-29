use serde::{Serialize};
use serde_json::{Result as JsonResult, Error};
use serde_json::Value;
use uuid::Uuid;

pub type SerializeError = Error;
pub type DeserializeError = ();


pub fn serialize<T>(value: &T) -> JsonResult<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec(value)
}

pub fn to_jsondoc<T>(_id: &Uuid, doc: &T) -> JsonResult<Value>
where
    T: Serialize,
{
    let mut json_value: Value = serde_json::to_value(doc).unwrap();
    json_value["_id"] = Value::String(_id.to_string());
    Ok(json_value.clone())
}

pub fn to_jsonresult<T>(_id: &Uuid, doc: &T) -> JsonResult<Value>
where
    T: Serialize,
{
    let mut json_value: Value = serde_json::to_value(doc).unwrap();
    json_value["data"]["_id"] = Value::String(_id.to_string());
    Ok(json_value["data"].clone())
}

// pub fn to_value<T>(doc: &T) -> Result<T, super::error::DStoreError>
// where
//     T: Deserialize,
// {
//     let mut json_value: Value = serde_json::to_value(doc).unwrap();
//     Ok(json_value)
// }
