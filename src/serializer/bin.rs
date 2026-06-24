use super::{FormatId, Serializer};
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct Bin;

#[cfg(feature = "bin_ser")]
impl Serializer for Bin {
    fn format_id(&self) -> FormatId {
        FormatId::Bin
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(bincode::serde::encode_to_vec(data, bincode::config::standard())?)
    }

    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let (val, _) = bincode::serde::decode_from_slice(data, bincode::config::standard())?;
        Ok(val)
    }
}

#[cfg(test)]
#[cfg(feature = "bin_ser")]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct S {
        x: u32,
        name: String,
    }

    #[test]
    fn round_trip() {
        let s = S {
            x: 5,
            name: "hello".into(),
        };
        let ser = Bin.serialize(&s).unwrap();
        let de: S = Bin.deserialize(&ser).unwrap();
        assert_eq!(de, s);
    }

    #[test]
    fn binary_payload_with_newline_byte_round_trips() {
        // Payload containing 0x0A (newline) — this was the v1 bug.
        let s = S {
            x: 10,
            name: "\nhidden\n".into(),
        };
        let ser = Bin.serialize(&s).unwrap();
        let de: S = Bin.deserialize(&ser).unwrap();
        assert_eq!(de, s);
    }

    #[test]
    fn no_trailing_newline() {
        let s = S {
            x: 1,
            name: "x".into(),
        };
        let bytes = Bin.serialize(&s).unwrap();
        assert_ne!(bytes.last().copied(), Some(b'\n'));
    }

    #[test]
    fn format_id_is_bin() {
        assert_eq!(Bin.format_id(), FormatId::Bin);
    }
}
