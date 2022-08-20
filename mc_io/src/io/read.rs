use std::io::{ErrorKind, Read, Write};
use binary::varint;
use crate::buf::Buffer;
use crate::error::{CommunicationError, ReadError};
use crate::{CompressionContext, ConnectionContext, FramedPacket, GlobalContext, MAXIMUM_PACKET_SIZE};
use crate::io::write;

const PROBE_LEN: usize = 2048;

pub fn read<S, H>(ctx: &mut GlobalContext, connection: &mut ConnectionContext<S>, mut handler: H) -> Result<(), CommunicationError>
where
    S: Read + Write,
    H: FnMut(&FramedPacket, &mut Buffer, CompressionContext) -> Result<(), CommunicationError>,
{
    let GlobalContext { read_buf, write_buf, compression_buf, compressor, decompressor} = ctx;
    let ConnectionContext { compression_threshold, socket, unwritten_buf, unread_buf, writeable } = connection;

    read_buf.reset();

    // Restore any bytes that weren't processed previously
    read_buf.copy_from(unread_buf.get_written());
    unread_buf.reset();

    while let ReadResult::Read(..) = socket_read(&mut *socket, read_buf)? {
        write_buf.reset();
        compression_buf.reset();

        while let DecodeResult::Packet(packet, network_len) = next_packet(read_buf.get_written())? {
            let compression_ctx = CompressionContext {
                compression_threshold: *compression_threshold,
                compression_buf,
                compressor,
                decompressor
            };

            (handler)(&packet, write_buf, compression_ctx)?;

            read_buf.consume(network_len);
        }

        write::write_buffer(&mut *socket, write_buf, unwritten_buf, writeable)?;
    }

    // Copy any unprocessed bytes into the `unread` buffer for future processing
    unread_buf.copy_from(read_buf.get_written());

    Ok(())
}

enum ReadResult {
    Read(usize),
    WouldBlock
}

fn socket_read<S: Read>(mut socket: S, buffer: &mut Buffer) -> Result<ReadResult, CommunicationError> {
    let unwritten = buffer.get_unwritten(PROBE_LEN);

    // Read the stream once
    let read = loop {
        match socket.read(unwritten) {
            Ok(0) => return Err(CommunicationError::Closed),
            Ok(amt) => break amt,
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok(ReadResult::WouldBlock),
            Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(CommunicationError::Io(err)),
        }
    };

    // SAFETY: We just put `read` bytes into the buffer
    unsafe {
        buffer.advance(read);
    }

    Ok(ReadResult::Read(read))
}

enum DecodeResult<'a> {
    Packet(FramedPacket<'a>, usize),
    Incomplete
}

fn next_packet(data: &[u8]) -> Result<DecodeResult, CommunicationError> {
    let available = data.len();

    if available >= 3 {
        let (packet_size, varint_header_bytes) = varint::decode::u21(data).map_err(|_| ReadError::VarInt)?;
        let packet_size = packet_size as usize;

        if packet_size > MAXIMUM_PACKET_SIZE {
            return Err(ReadError::PacketTooLarge.into());
        }

        if available >= varint_header_bytes + packet_size {
            let packet = &data[varint_header_bytes..][..packet_size];
            let network_len = varint_header_bytes + packet_size;

            Ok(DecodeResult::Packet(FramedPacket(packet), network_len))
        } else {
            Ok(DecodeResult::Incomplete)
        }
    } else if available == 2 && data[0] == 1 {
        Ok(DecodeResult::Packet(FramedPacket(&data[1..2]), 2))
    } else if available == 1 && data[0] == 0 {
        Err(ReadError::ZeroSizedPacket.into())
    } else {
        Ok(DecodeResult::Incomplete)
    }
}
