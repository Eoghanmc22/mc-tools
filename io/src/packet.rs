mod lazy_varint;

use std::fmt::Debug;
use binary::slice_serialization::SliceSerializable;
use libdeflater::Compressor;
use protocol::IdentifiedPacket;
use crate::buf::Buffer;
use crate::read::RawPacket;

pub struct WriteContext<'a, 'b> {
    pub compression_threshold: i32,
    pub packet_buf: &'a mut Buffer,
    pub compression_buf: &'a mut Buffer,
    pub compressor: &'b mut Compressor
}

pub fn write_packet<'a, 'b, I: Debug, T>(packet: &'b T, ctx: WriteContext<'a, '_>) -> &'a [u8]
where
    T: SliceSerializable<'b, T> + IdentifiedPacket<I> + 'b,
{
    todo!()
}

pub fn read_packet(packet: RawPacket) {

}

/*fn handle_uncompressed_frame(data: &[u8]) -> Result<RawPacket, CommunicationError> {
    Ok(RawPacket(data))
}

fn handle_compressed_frame<'a>(mut data: &'a [u8], compression_buffer: &'a mut Buffer, decompressor: &mut Decompressor, compression_threshold: i32) -> Result<RawPacket<'a>, CommunicationError> {
    let data_len = slice_serialization::VarInt::read(&mut data).context("data len read")? as usize;

    // Handle packets too small to be compressed
    if data_len == 0 {
        return Ok(RawPacket(data));
    }

    if data_len > MAXIMUM_PACKET_SIZE {
        return Err(anyhow!("Uncompressed packet size exceeded limit").into());
    }
    if data_len < compression_threshold as usize {
        return Err(anyhow!("Uncompressed packet size is below compression_threshold").into());
    }

    // Decompress the packet
    compression_buffer.reset();
    let unwritten = compression_buffer.get_unwritten(data_len);
    let decompressed = decompressor.zlib_decompress(data, unwritten)?;

    if decompressed != data_len {
        return Err(anyhow!("Decompressed size {decompressed} is not equal to the size received in header {data_len}").into());
    }

    // SAFETY: we just put `decompressed` bytes into the buffer
    unsafe {
        compression_buffer.advance(decompressed);
    }

    Ok(RawPacket(compression_buffer.get_written()))
}*/
