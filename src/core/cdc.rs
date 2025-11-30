use fastcdc::v2020::FastCDC;

#[derive(Debug, Clone, Copy)]
pub struct CdcParams {
    pub min_size: usize,
    pub avg_size: usize,
    pub max_size: usize,
}

#[derive(Debug)]
pub struct Chunk {
    pub offset: usize,
    pub length: usize,
    pub data: Vec<u8>,
}

pub fn chunk_data(data: &[u8], params: CdcParams) -> Vec<Chunk> {
    let chunker = FastCDC::new(data, params.min_size, params.avg_size, params.max_size);

    chunker.map(|chunk| {
            Chunk {
                offset: chunk.offset,
                length: chunk.length,
                data: data[chunk.offset .. (chunk.offset + chunk.length)].to_vec(),
            }
        }).collect()
}