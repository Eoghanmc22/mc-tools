use proto::Packet;

use crate::buf::Buffer;
use crate::error::{ReadError, WriteError};
use crate::packet::lazy_varint::LazyVarint;
use crate::{
    CompressionReadContext, CompressionWriteContext, FramedPacket, RawPacket, MAXIMUM_PACKET_SIZE,
};

use super::compression;

struct PacketMeta<'a> {
    write_buf: &'a mut [u8],
    packet_type: PacketType<'a>,
    header_len: usize,
}

enum PacketType<'a> {
    Compressed(LazyVarint<'a, 3>, LazyVarint<'a, 3>, Option<&'a mut Buffer>),
    Normal(LazyVarint<'a, 3>),
}

// Expected size should include packet id byte
fn create_packet_meta<'a>(
    packet_buf: &'a mut Buffer,
    compression_buf: &'a mut Buffer,
    expected_size: usize,
    compression_threshold: i32,
) -> PacketMeta<'a> {
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

        let total_len = LazyVarint::new(&mut write_buf);
        let data_len = LazyVarint::new(&mut write_buf);

        let packet_type = PacketType::Compressed(total_len, data_len, dst);

        PacketMeta {
            write_buf,
            packet_type,
            header_len,
        }
    } else {
        let header_len = 3;
        let mut write_buf = packet_buf.get_unwritten(header_len + expected_size);

        let total_len = LazyVarint::new(&mut write_buf);
        let packet_type = PacketType::Normal(total_len);

        PacketMeta {
            write_buf,
            packet_type,
            header_len,
        }
    }
}

pub fn write_packet<'a, 'b, P>(
    packet: &'a P,
    packet_buf: &mut Buffer,
    ctx: CompressionWriteContext,
    compression_threshold: i32,
) -> Result<(), WriteError>
where
    P: Packet<'a>,
{
    let CompressionWriteContext {
        compression_buf,
        compressor,
        ..
    } = ctx;
    compression_buf.reset();

    let expected_packet_size = packet.expected_size();
    if expected_packet_size > MAXIMUM_PACKET_SIZE {
        return Err(WriteError::PacketTooLarge);
    }

    let PacketMeta {
        write_buf,
        packet_type,
        header_len,
    } = create_packet_meta(
        packet_buf,
        compression_buf,
        1 + expected_packet_size,
        compression_threshold,
    );

    let pre_write_len = write_buf.len();

    write_buf[0] = P::PACKET_ID_NUM;
    let slice_after_write = packet.encode(&mut write_buf[1..]);

    let packet_size = pre_write_len - slice_after_write.len();

    match packet_type {
        PacketType::Compressed(total_len, data_len, dst) => {
            total_len.write(3 + packet_size as i32);
            data_len.write(0);

            if let Some(dst) = dst {
                if packet_size >= compression_threshold as usize {
                    compression::compress(&write_buf[..packet_size], dst, compressor)?;
                } else {
                    // SAFETY: We wrote a full packet into compression_buf
                    let data = unsafe { compression_buf.advance_write(header_len + packet_size) };
                    packet_buf.copy_from(data);
                }
            } else {
                // SAFETY: We wrote a full packet into packet_buf
                unsafe {
                    packet_buf.advance_write(header_len + packet_size);
                }
            }
        }
        PacketType::Normal(total_len) => {
            total_len.write(packet_size as i32);

            // SAFETY: We wrote a full packet into packet_buf
            unsafe {
                packet_buf.advance_write(header_len + packet_size);
            }
        }
    }

    Ok(())
}

pub fn read_packet<'a>(
    packet: &'a FramedPacket,
    ctx: CompressionReadContext<'a, '_>,
    compression_threshold: i32,
) -> Result<RawPacket<'a>, ReadError> {
    let CompressionReadContext {
        compression_buf,
        decompressor,
        ..
    } = ctx;

    let buffer = if compression_threshold > 0 {
        compression::decompress(
            packet.0,
            compression_buf,
            decompressor,
            compression_threshold,
        )?
    } else {
        packet.0
    };

    Ok(RawPacket(buffer))
}
