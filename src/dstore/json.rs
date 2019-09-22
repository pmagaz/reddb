
use serde_json::Result as JsonResult;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub fn serialize<T>(type: T, content: &str) -> JsonResult<T>{
    let doc: T = serde_json::from_str(content).unwrap();
}
