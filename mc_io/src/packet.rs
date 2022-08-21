use crate::buf::Buffer;
use crate::error::{ReadError, WriteError};
use crate::{CompressionContext, FramedPacket, MAXIMUM_PACKET_SIZE, RawPacket};
use binary::slice_serialization::{SliceSerializable, VarInt};
use protocol::IdentifiedPacket;
use std::fmt::Debug;
use crate::packet::lazy_varint::LazyVarint;

mod compression;
mod lazy_varint;

struct PacketMeta<'a> {
    write_buf: &'a mut [u8],
    packet_type: PacketType<'a>,
    header_len: usize,
}

enum PacketType<'a> {
    Compressed(LazyVarint<'a>, LazyVarint<'a>, Option<&'a mut Buffer>),
    Normal(LazyVarint<'a>),
}

// Expected size should include packet id byte
fn create_packet_meta<'a>(packet_buf: &'a mut Buffer, compression_buf: &'a mut Buffer, expected_size: usize, compression_threshold: i32) -> PacketMeta<'a> {
    if compression_threshold > 0 {
        let header_len = 6;

        let (mut write_buf, dst) = {
            if expected_size > compression_threshold as usize {
                let write_buf = compression_buf.get_unwritten(header_len + expected_size);
                let dst = Some(packet_buf);

                (write_buf, dst)
            } else {
                let write_buf = packet_buf.get_unwritten(header_len + expected_size);
                let dst = None;

                (write_buf, dst)
            }
        };

        let total_len = LazyVarint::new(&mut write_buf, 3);
        let data_len = LazyVarint::new(&mut write_buf, 3);

        let packet_type = PacketType::Compressed(total_len, data_len, dst);

        PacketMeta {
            write_buf,
            packet_type,
            header_len
        }
    } else {
        let header_len = 3;
        let mut write_buf = packet_buf.get_unwritten(header_len + expected_size);

        let total_len = LazyVarint::new(&mut write_buf, 3);
        let packet_type = PacketType::Normal(total_len);

        PacketMeta {
            write_buf,
            packet_type,
            header_len
        }
    }
}

pub fn write_packet<'a, 'b, I: Debug, T>(
    packet: &'a T,
    packet_buf: &mut Buffer,
    ctx: &mut CompressionContext,
) -> Result<(), WriteError>
where
    T: SliceSerializable<'a, T> + IdentifiedPacket<I> + 'a,
{
    let CompressionContext {
        compression_threshold,
        compression_buf,
        compressor,
        ..
    } = ctx;
    compression_buf.reset();

    let expected_packet_size = T::get_write_size(T::maybe_deref(packet));
    if expected_packet_size > MAXIMUM_PACKET_SIZE {
        return Err(WriteError::PacketTooLarge);
    }

    let PacketMeta { write_buf, packet_type, header_len } = create_packet_meta(packet_buf, compression_buf, 1 + expected_packet_size, *compression_threshold);
    write_buf[0] = packet.get_packet_id_as_u8();

    // SAFETY: We allocated at least `T::get_write_size` bytes
    let slice_after_write = unsafe { T::write(&mut write_buf[1..], T::maybe_deref(packet)) };
    let packet_size = 1 + expected_packet_size - slice_after_write.len();

    match packet_type {
        PacketType::Compressed(total_len, data_len, dst) => {
            total_len.write(3 + packet_size as i32);
            data_len.write(0);

            if let Some(dst) = dst {
                if packet_size >= *compression_threshold as usize {
                    compression::compress(&write_buf[..packet_size], dst, compressor)?;
                } else {
                    // SAFETY: We wrote a full packet into compression_buf
                    let data = unsafe { compression_buf.advance(header_len + packet_size) };
                    packet_buf.copy_from(data);
                }
            } else {
                // SAFETY: We wrote a full packet into packet_buf
                unsafe { packet_buf.advance(header_len + packet_size); }
            }
        }
        PacketType::Normal(total_len) => {
            total_len.write(packet_size as i32);

            // SAFETY: We wrote a full packet into packet_buf
            unsafe { packet_buf.advance(header_len + packet_size); }
        }
    }

    Ok(())
}

pub fn read_packet<'a>(packet: &'a FramedPacket, ctx: &'a mut CompressionContext) -> Result<RawPacket<'a>, ReadError> {
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
    Ok(RawPacket(packet_id as u8, buffer))
}
