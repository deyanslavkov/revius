use std::io;
use zstd;
use crate::error::ReviusError;

pub fn compress(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    zstd::encode_all(data, level)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, ReviusError> {
    zstd::decode_all(data)
        .map_err(|e| ReviusError::Db(format!("Blob decompression failed (corrupt?): {}", e)))
}