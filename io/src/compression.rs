use binary::slice_serialization::{VarInt, SliceSerializable};
use libdeflater::{Compressor, Decompressor};

use crate::{buf::Buffer, packet::lazy_varint::LazyVarint, error::{WriteError, ReadError}};

pub fn compress<'a>(src: &[u8], dst: &'a mut Buffer, compressor: &mut Compressor) -> Result<&'a [u8], WriteError> {
    let max_compressed_size = compressor.zlib_compress_bound(src.len());

    let buffer = dst.get_unwritten(3 + 3 + max_compressed_size);

    let (total_len, buffer) = LazyVarint::new(buffer, 3);
    let (data_len, buffer) = LazyVarint::new(buffer, 3);

    let compressed = compressor.zlib_compress(src, buffer)?;

    data_len.write(src.len() as i32);
    total_len.write(3 + compressed as i32);

    Ok(unsafe { dst.advance(3 + 3 + compressed) })
}


const MAXIMUM_PACKET_SIZE: usize = 2097148;

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
