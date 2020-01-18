use serde::Serialize;
use serde_json::Value;
use serde_json::{Error, Result as JsonResult};
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

//pub fn to_jsonstring<'a, T>(doc: &T) -> String
// pub fn to_jsonstring<'a, T>(doc: &T) -> &'a [u8]
// where
//     T: Serialize + Sized,
// {
//     let json_string = serde_json::to_string(&doc).unwrap();
//     json_string.as_bytes()
//     //json_string.clone().as_bytes()
//     //leches
//     //let string = &json_string.clone();
//     //string.as_bytes()
//     //json_string
// }

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
