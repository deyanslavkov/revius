use crate::error::ReviusError;
use std::io::Cursor;

pub fn compress(data: &[u8], level: i32) -> Result<Vec<u8>, ReviusError> {
    zstd::stream::encode_all(Cursor::new(data), level)
        .map_err(|e| ReviusError::Compression(e.to_string()))
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, ReviusError> {
    zstd::stream::decode_all(Cursor::new(data))
        .map_err(|e| ReviusError::Compression(e.to_string()))
}