use fastcdc::v2020::FastCDC;

pub fn chunk_data(data: &[u8], min_size: u32, avg_size: u32, max_size: u32) -> Vec<&[u8]> {
    let chunker = FastCDC::new(data, min_size, avg_size, max_size);
    chunker.map(|chunk| &data[chunk.offset..chunk.offset + chunk.length]).collect()
}