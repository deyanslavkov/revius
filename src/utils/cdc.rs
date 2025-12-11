use fastcdc::v2020::FastCDC;

pub fn chunk_data(data: &[u8], min_size: u64, avg_size: u64, max_size: u64) -> Vec<&[u8]> {
    let chunker = FastCDC::new(data, min_size as u32, avg_size as u32, max_size as u32);
    chunker.map(|chunk| &data[chunk.offset..chunk.offset + chunk.length]).collect()
}