use crate::error::CommunicationError;
use crate::{buf::Buffer, ConnectionWriteContext};
use std::io::{ErrorKind, Write};
use std::ops::Deref;

pub(crate) fn write_unwritten<D, S>(
    connection: &mut ConnectionWriteContext<D>,
) -> Result<(), CommunicationError>
where
    D: Deref<Target = S>,
    for<'a> &'a S: Write,
{
    let ConnectionWriteContext {
        socket,
        unwritten_buf: unwritten,
        writeable,
        ..
    } = connection;
    *writeable = true;

    if unwritten.is_empty() {
        return Ok(());
    }

    let consumed = write_buf(D::deref(socket), unwritten.get_written(), writeable)?;
    unwritten.consume(consumed);

    Ok(())
}

pub(crate) fn write_slice<S>(
    socket: S,
    mut to_write: &[u8],
    unwritten: &mut Buffer,
    writeable: &mut bool,
) -> Result<(), CommunicationError>
where
    S: Write,
{
    if to_write.is_empty() {
        return Ok(());
    }

    let consumed = write_buf(socket, to_write, writeable)?;
    to_write = &to_write[consumed..];
    unwritten.copy_from(to_write);

    Ok(())
}

fn write_buf<S>(
    mut socket: S,
    mut buffer: &[u8],
    writeable: &mut bool,
) -> Result<usize, CommunicationError>
where
    S: Write,
{
    if !*writeable || buffer.is_empty() {
        return Ok(0);
    }

    let mut consume = 0;
    loop {
        match socket_write(&mut socket, buffer)? {
            WriteResult::Write(new_buffer, consumed) => {
                buffer = new_buffer;
                consume += consumed;
            }
            WriteResult::WouldBlock => {
                *writeable = false;
                break;
            }
            WriteResult::Empty => break,
        }
    }
    Ok(consume)
}

enum WriteResult<'a> {
    Write(&'a [u8], usize),
    WouldBlock,
    Empty,
}

fn socket_write<S: Write>(mut socket: S, buffer: &[u8]) -> Result<WriteResult, CommunicationError> {
    if buffer.is_empty() {
        return Ok(WriteResult::Empty);
    }

    // Write to the stream once
    loop {
        match socket.write(buffer) {
            Ok(0) => return Err(CommunicationError::Closed),
            Ok(amt) => break Ok(WriteResult::Write(&buffer[amt..], amt)),
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(WriteResult::WouldBlock)
            }
            Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(CommunicationError::Io(err)),
        }
    }
}
