use std::io::{ErrorKind, Read, Write};
use crate::ctx::ConnectionContext;
use crate::error::CommunicationError;

pub fn write<S>(connection: &mut ConnectionContext<S>) -> Result<(), CommunicationError>
    where
        S: Read + Write,
{
    let ConnectionContext { socket, unwritten, writeable, .. } = connection;

    let consumed = write_buf(socket, &unwritten[..], writeable)?;
    unwritten.drain(..consumed);

    Ok(())
}

pub fn write_slice<S>(socket: S, to_write: &[u8], unwritten: &mut Vec<u8>, writeable: &mut bool) -> Result<(), CommunicationError>
    where
        S: Read + Write,
{
    let consumed = write_buf(socket, to_write, writeable)?;
    unwritten.extend_from_slice(&to_write[consumed..]);

    Ok(())
}

fn write_buf<S>(mut socket: S, mut buffer: &[u8], writeable: &mut bool) -> Result<usize, CommunicationError>
    where
        S: Read + Write,
{
    if *writeable && !buffer.is_empty() {
        let mut consume = 0;
        loop {
            match socket_write(&mut socket, buffer)? {
                WriteResult::Write(new_buffer, consumed) => {
                    buffer = new_buffer;
                    consume += consumed;
                }
                WriteResult::WouldBlock => {
                    *writeable = false;
                    break
                }
                WriteResult::Empty => break
            }
        }
        Ok(consume)
    } else {
        Ok(0)
    }
}

enum WriteResult<'a> {
    Write(&'a [u8], usize),
    WouldBlock,
    Empty
}

fn socket_write<S: Write>(mut socket: S, buffer: &[u8]) -> Result<WriteResult, CommunicationError> {
    if buffer.is_empty() {
        return Ok(WriteResult::Empty);
    }

    // Write to the stream once
    loop {
        match socket.write(buffer) {
            Ok(0) => return Err(CommunicationError::Closed),
            Ok(amt) => break Ok(WriteResult::Write(&buffer[..amt], amt)),
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok(WriteResult::WouldBlock),
            Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(CommunicationError::Io(err)),
        }
    }
}
