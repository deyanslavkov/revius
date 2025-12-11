use std::io;
use zstd;

pub fn compress(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    zstd::encode_all(data, level)
}

pub fn decompress(data: &[u8]) -> io::Result<Vec<u8>> {
    zstd::decode_all(data)
}