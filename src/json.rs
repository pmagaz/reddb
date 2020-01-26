use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::{Error, Result as JsonResult};
use uuid::Uuid;

use super::document::Document;

pub type SerializeError = Error;
pub type DeserializeError = ();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonDocument {
    pub _id: Value,
    pub data: Value,
}

pub fn serialize<T>(value: &T) -> JsonResult<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec(value)
}

pub fn from_str(value: &str) -> JsonResult<Value> {
    let json_doc: Value = serde_json::from_str(value)?;
    Ok(json_doc)
}

pub fn to_jsonlog<T>(_id: &Uuid, doc: &T) -> JsonResult<Value>
where
    T: Serialize,
{
    let mut json_value = serde_json::to_value(doc)?;
    json_value["_id"] = Value::String(_id.to_string());
    //json_value["_status"] = Value::String();
    Ok(json_value.clone())
}

// pub fn to_jsonresult(_id: &Uuid, doc: &Document) -> JsonResult<Value> {
//     let mut json_value: Value = serde_json::to_value(doc)?;
//     json_value["data"]["_id"] = Value::String(_id.to_string());
//     // Ok(json_value["data"])
//     Ok(json_value["data"].clone())
// }

// pub fn to_jsonresult2<'a, T>(_id: &Uuid, doc: &T) -> JsonResult<&'a Value>
// where
//     T: Serialize,
// {
//     let mut json_value: Value = serde_json::to_value(doc)?;
//     json_value["data"]["_id"] = Value::String(_id.to_string());
//     Ok(&json_value["data"])
//     //Ok(json_value["data"].clone())
// }

pub fn _to_operationlog(_id: &Uuid, doc: &Document) -> JsonResult<Value> {
    let mut json_value: Value = serde_json::to_value(doc)?;
    json_value["data"]["_id"] = Value::String(_id.to_string());
    Ok(json_value["data"].clone())
}
