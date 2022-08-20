use crate::buf::Buffer;
use crate::error::{ReadError, WriteError};
use crate::{CompressionContext, FramedPacket, MAXIMUM_PACKET_SIZE, RawPacket};
use binary::slice_serialization::{SliceSerializable, VarInt};
use protocol::IdentifiedPacket;
use std::fmt::Debug;
use crate::packet::lazy_varint::LazyVarint;

mod compression;
mod lazy_varint;

pub fn write_packet<'a, 'b, I: Debug, T>(
    packet: &'a T,
    packet_buf: &'b mut Buffer,
    ctx: &'b mut CompressionContext<'b, '_, '_>,
) -> Result<&'b [u8], WriteError>
where
    T: SliceSerializable<'a, T> + IdentifiedPacket<I> + 'a,
{
    let CompressionContext {
        compression_threshold,
        compression_buf,
        compressor,
        ..
    } = ctx;

    let expected_packet_size = T::get_write_size(T::maybe_deref(packet));
    if expected_packet_size > MAXIMUM_PACKET_SIZE {
        return Err(WriteError::PacketTooLarge);
    }

    let mut buffer = packet_buf.get_unwritten(3 + 3 + 1 + expected_packet_size);

    let len1 = LazyVarint::new(&mut buffer, 3);
    let len2 = LazyVarint::new(&mut buffer, 3);
    buffer[0] = packet.get_packet_id_as_u8();

    // SAFETY: We allocated at least `T::get_write_size` bytes
    let slice_after_write = unsafe { T::write(&mut buffer[1..], T::maybe_deref(packet)) };
    let packet_size = 1 + expected_packet_size - slice_after_write.len();

    if *compression_threshold > 0 {
        if packet_size >= *compression_threshold as usize {
            Ok(compression::compress(buffer, compression_buf, compressor)?)
        } else {
            len1.write(3 + packet_size as i32);
            len2.write(0);

            Ok(unsafe { packet_buf.advance(3 + 3 + packet_size) })
        }
    } else {
        len2.write(packet_size as i32);

        Ok(&unsafe { packet_buf.advance(3 + 3 + packet_size) }[3..])
    }
}

pub fn read_packet<'a>(packet: FramedPacket<'a>, ctx: &'a mut CompressionContext<'a, '_, '_>) -> Result<RawPacket<'a>, ReadError> {
    let CompressionContext {
        compression_threshold,
        compression_buf,
        decompressor,
        ..
    } = ctx;

    let mut buffer = if *compression_threshold > 0 {
        compression::decompress(packet.0, compression_buf, decompressor, *compression_threshold)?
    } else {
        packet.0
    };

    let packet_id = VarInt::read(&mut buffer).map_err(|_| ReadError::VarInt)?;
    Ok(RawPacket(packet_id, buffer))
}
