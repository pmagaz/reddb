use super::{FormatId, Serializer};
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct Json;

#[cfg(feature = "json_ser")]
impl Serializer for Json {
    fn format_id(&self) -> FormatId {
        FormatId::Json
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        // Trailing \n required while storage uses line-based reading.
        let mut bytes = serde_json::to_vec(data)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(serde_json::from_slice(data)?)
    }
}

#[cfg(test)]
#[cfg(feature = "json_ser")]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct S {
        x: u32,
    }

    #[test]
    fn round_trip() {
        let s = S { x: 42 };
        let ser = Json.serialize(&s).unwrap();
        let de: S = Json.deserialize(&ser).unwrap();
        assert_eq!(de, s);
    }

    #[test]
    fn format_id_is_json() {
        assert_eq!(Json.format_id(), FormatId::Json);
    }
}
