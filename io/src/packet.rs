pub(crate) mod lazy_varint;

use crate::buf::Buffer;
use crate::compression::compress;
use crate::error::WriteError;
use crate::read::RawPacket;
use binary::slice_serialization::SliceSerializable;
use libdeflater::Compressor;
use protocol::IdentifiedPacket;
use std::fmt::Debug;

use self::lazy_varint::LazyVarint;

pub struct WriteContext<'a, 'b> {
    pub compression_threshold: i32,
    pub packet_buf: &'a mut Buffer,
    pub compression_buf: &'a mut Buffer,
    pub compressor: &'b mut Compressor,
}

const MAXIMUM_PACKET_SIZE: usize = 2097148;

pub fn write_packet<'a, 'b, I: Debug, T>(
    packet: &'a T,
    ctx: WriteContext<'b, '_>,
) -> Result<&'b [u8], WriteError>
where
    T: SliceSerializable<'a, T> + IdentifiedPacket<I> + 'a,
{
    let WriteContext {
        compression_threshold,
        packet_buf,
        compression_buf,
        compressor,
    } = ctx;

    let expected_packet_size = T::get_write_size(T::maybe_deref(packet));
    if expected_packet_size > MAXIMUM_PACKET_SIZE {
        return Err(WriteError::PacketTooLarge);
    }

    let buffer = packet_buf.get_unwritten(3 + 3 + 1 + expected_packet_size);

    let (len1, buffer) = LazyVarint::new(buffer, 3);
    let (len2, buffer) = LazyVarint::new(buffer, 3);
    buffer[0] = packet.get_packet_id_as_u8();

    // SAFETY: We allocated at least `T::get_write_size` bytes
    let slice_after_write = unsafe { T::write(&mut buffer[1..], T::maybe_deref(packet)) };
    let bytes_written = 1 + expected_packet_size - slice_after_write.len();

    if compression_threshold > 0 {
        if bytes_written >= compression_threshold as usize {
            Ok(compress(buffer, compression_buf, compressor)?)
        } else {
            len1.write(3 + bytes_written as i32);
            len2.write(0);

            Ok(unsafe { packet_buf.advance(3 + 3 + bytes_written) })
        }
    } else {
        len2.write(bytes_written as i32);

        Ok(&unsafe { packet_buf.advance(3 + 3 + bytes_written) }[3..])
    }
}

pub fn read_packet(packet: RawPacket) {}

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
