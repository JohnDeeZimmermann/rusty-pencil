//! Encoded, content-addressed images. Crop coordinates are fractions of the source image.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FileId(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryFile {
    pub mime: String,
    #[serde(with = "encoded")]
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
impl BinaryFile {
    pub fn from_bytes(data: Vec<u8>) -> Option<Self> {
        let mime = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
            "image/jpeg"
        } else {
            return None;
        };
        let size = imagesize::blob_size(&data).ok()?;
        let file = Self {
            mime: mime.into(),
            data,
            width: u32::try_from(size.width).ok()?,
            height: u32::try_from(size.height).ok()?,
        };
        file.valid().then_some(file)
    }
    pub fn id(&self) -> FileId {
        FileId(blake3::hash(&self.data).to_hex().to_string())
    }
    pub fn valid(&self) -> bool {
        ((self.mime == "image/png" && self.data.starts_with(b"\x89PNG\r\n\x1a\n"))
            || (self.mime == "image/jpeg" && self.data.starts_with(&[0xff, 0xd8, 0xff])))
            && imagesize::blob_size(&self.data).is_ok_and(|size| {
                size.width == self.width as usize && size.height == self.height as usize
            })
            && !self.data.is_empty()
            && self.data.len() <= 16 * 1024 * 1024
            && self.width > 0
            && self.height > 0
            && u64::from(self.width) * u64::from(self.height) <= 16 * 1024 * 1024
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Default for Crop {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}
impl Crop {
    pub fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width >= 0.0001
            && self.height >= 0.0001
            && self.x + self.width <= 1.00001
            && self.y + self.height <= 1.00001
    }
}

mod encoded {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};
    pub(super) fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(data))
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        if encoded.len() > 24 * 1024 * 1024 {
            return Err(serde::de::Error::custom("embedded image exceeds 16 MiB"));
        }
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}
