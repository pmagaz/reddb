use serde_json::Value;

use serde::de::DeserializeOwned;
use serde_json::Error;
use serde_json::Result as YamlResult;
use std::collections::HashMap;
//use serde::ser::Serializer;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub createdAt: String,
    pub updatedAt: String,
    pub documents: Vec<Document>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub data: String,
}
pub type Repr = String;
pub type SerializeError = Error;
pub type DeserializeError = ();

pub fn serialize<T>(value: &T) -> YamlResult<String>
where
    T: Serialize,
{
    serde_json::to_string(value)
}

// pub fn deserialize2<'de, D>(deserializer: D) -> Result<HashMap<i64, Document>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let mut map = HashMap::new();
//     for item in Vec::<Document>::deserialize(deserializer)? {
//         map.insert(item.id, item);
//     }
//     Ok(map)
// }

pub fn deserialize<T, I: AsRef<[u8]>>(bytes: &I) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let data: Value = serde_json::from_slice(bytes.as_ref()).unwrap();
    let documents: &Vec<Value> = data["documents"].as_array().unwrap();
    // let mut map = HashMap::new();
    // for item in Vec::<Document>::deserialize(deserializer)? {
    //     map.insert(item.id, item);
    // }
    //let documents: &Vec<Document> = &data["documents"].as_array().unwrap();
    //let documents: &Vec<Value> = &bytes["documents"].as_array().unwrap();
    let string = String::from_utf8(bytes.as_ref().to_vec()).unwrap();
    let des = serde_json::from_str(&string).unwrap();
    Ok(des)
}
