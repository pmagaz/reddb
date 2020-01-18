use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::{Error, Result as JsonResult};
use uuid::Uuid;

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

pub fn from_str(value: &str) -> JsonResult<JsonDocument> {
    let json_doc: JsonDocument = serde_json::from_str(value)?;
    Ok(json_doc)
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
