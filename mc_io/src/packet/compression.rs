use binary::slice_serialization::{VarInt, SliceSerializable};
use libdeflater::{Compressor, Decompressor};

use crate::{buf::Buffer, error::{WriteError, ReadError}, MAXIMUM_PACKET_SIZE};
use crate::packet::lazy_varint::LazyVarint;

pub fn compress<'a>(src: &[u8], dst: &'a mut Buffer, compressor: &mut Compressor) -> Result<&'a [u8], WriteError> {
    let max_compressed_size = compressor.zlib_compress_bound(src.len());

    let mut buffer = dst.get_unwritten(3 + 3 + max_compressed_size);

    let total_len = LazyVarint::new(&mut buffer, 3);
    let data_len = LazyVarint::new(&mut buffer, 3);

    let compressed = compressor.zlib_compress(src, buffer)?;

    data_len.write(src.len() as i32);
    total_len.write(3 + compressed as i32);

    Ok(unsafe { dst.advance(3 + 3 + compressed) })
}


pub fn decompress<'a>(mut src: &[u8], dst: &'a mut Buffer, decompressor: &mut Decompressor, compression_threshold: i32) -> Result<&'a [u8], ReadError> {
    let data_len = VarInt::read(&mut src).map_err(|_| ReadError::VarInt)? as usize;
    
    if data_len > MAXIMUM_PACKET_SIZE {
        return Err(ReadError::PacketTooLarge);
    }

    if data_len < compression_threshold as usize {
        return Err(ReadError::BadlyCompressed);
    }

    let buffer = dst.get_unwritten(data_len);

    let decompressed = decompressor.zlib_decompress(src, buffer)?;

    if decompressed != data_len {
        return Err(ReadError::BadlyCompressed);
    }

    Ok(unsafe { dst.advance(decompressed) })
}


#[cfg(test)]
mod tests {
    use binary::slice_serialization::{SliceSerializable, VarInt};
    use libdeflater::{CompressionLvl, Compressor, Decompressor};
    use crate::Buffer;
    use super::*;

    #[test]
    fn compression_roundtrip() {
        let mut compressor = Compressor::new(CompressionLvl::best());
        let mut decompressor = Decompressor::new();

        do_compression_roundtrip::<10>(&mut compressor, &mut decompressor);
        do_compression_roundtrip::<100>(&mut compressor, &mut decompressor);
        do_compression_roundtrip::<1000>(&mut compressor, &mut decompressor);
        do_compression_roundtrip::<10000>(&mut compressor, &mut decompressor);
    }

    fn do_compression_roundtrip<const DATA_SIZE: usize>(compressor: &mut Compressor, decompressor: &mut Decompressor) {
        let original: [u8; DATA_SIZE] = rand::random();

        let mut compression_buffer = Buffer::with_capacity(6 + DATA_SIZE);
        let mut compressed = compress(&original, &mut compression_buffer, compressor).unwrap();
        let compressed_len = compressed.len();

        let total_size = VarInt::read(&mut compressed).unwrap();
        assert_eq!(total_size as usize, compressed.len());

        let mut decompression_buffer = Buffer::with_capacity(6 + DATA_SIZE);
        let decompressed = decompress(compressed, &mut decompression_buffer, decompressor, 1).unwrap();
        assert_eq!(original, decompressed);
        assert_eq!(compression_buffer.len(), compressed_len);
    }
}