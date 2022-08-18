pub(crate) mod lazy_varint;

use crate::buf::Buffer;
use crate::error::{ReadError, WriteError};
use crate::read::FramedPacket;
use binary::slice_serialization::{SliceSerializable, VarInt};
use libdeflater::{Compressor, Decompressor};
use protocol::IdentifiedPacket;
use std::fmt::Debug;
use crate::compression;

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
            Ok(compression::compress(buffer, compression_buf, compressor)?)
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

pub struct ReadContext<'a, 'b> {
    pub compression_threshold: i32,
    pub compression_buf: &'a mut Buffer,
    pub decompressor: &'b mut Decompressor,
}

#[derive(Debug)]
pub struct RawPacket<'a>(pub i32, pub &'a [u8]);

pub fn read_packet<'a>(mut packet: FramedPacket<'a>, ctx: ReadContext<'a, '_>) -> Result<RawPacket<'a>, ReadError> {
    let ReadContext { compression_threshold, compression_buf, decompressor } = ctx;

    let mut buffer = if compression_threshold > 0 {
        compression::decompress(packet.0, compression_buf, decompressor, compression_threshold)?
    } else {
        packet.0
    };

    let packet_id = VarInt::read(&mut buffer).map_err(|_| ReadError::VarInt)?;
    Ok(RawPacket(packet_id, buffer))
}
