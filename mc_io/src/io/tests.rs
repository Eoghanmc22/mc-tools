use super::{write::{write, write_buffer}, read::read};
use helpers::{Chunked, EOF2WB};

mod helpers {
    use std::io::{ErrorKind, Read, Write, self};

    pub struct EOF2WB<I: Read + Write>(pub I);

    impl<I: Read + Write> Read for EOF2WB<I> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self.0.read(buf) {
                Ok(0) => Err(io::Error::new(ErrorKind::WouldBlock, "")),
                other => other
            }
        }
    }

    impl<I: Read + Write> Write for EOF2WB<I> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self.0.write(buf) {
                Ok(0) => Err(io::Error::new(ErrorKind::WouldBlock, "")),
                other => other
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }

    pub struct Chunked<I: Read + Write>(pub I, pub usize);

    impl<I: Read + Write> Read for Chunked<I> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let len = usize::min(buf.len(), self.1);
            self.0.read(&mut buf[..len])
        }
    }

    impl<I: Read + Write> Write for Chunked<I> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let len = usize::min(buf.len(), self.1);
            self.0.write(&buf[..len])
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
}

mod write {
    use std::io::Cursor;
    use rand::Rng;
    use crate::{Buffer, ConnectionContext};
    use super::*;

    #[test]
    fn normal_write() {
        let mut socket = EOF2WB(Chunked(Cursor::new(Vec::with_capacity(1000)), 37));
        let mut buffer = Buffer::with_capacity(1000);
        let mut unwritten = Buffer::with_capacity(0);

        let raw_buffer = buffer.get_unwritten(1000);
        rand::thread_rng().fill(raw_buffer);
        let written = unsafe { buffer.advance(1000) }.to_vec();

        let mut writeable = true;
        write_buffer(&mut socket, &mut buffer, &mut unwritten, &mut writeable).unwrap();

        assert!(writeable);
        assert!(buffer.is_empty());
        assert!(unwritten.is_empty());
        assert_eq!(socket.0.0.into_inner(), written);
    }

    #[test]
    fn buffered_write() {
        let mut socket = EOF2WB(Chunked(Cursor::new([0u8; 500]), 37));
        let mut buffer = Buffer::with_capacity(1000);
        let mut unwritten = Buffer::with_capacity(500);

        let raw_buffer = buffer.get_unwritten(1000);
        rand::thread_rng().fill(raw_buffer);
        let written = unsafe { buffer.advance(1000) }.to_vec();

        let mut writeable = true;
        write_buffer(&mut socket, &mut buffer, &mut unwritten, &mut writeable).unwrap();

        assert!(!writeable);
        assert!(buffer.is_empty());
        assert_eq!(socket.0.0.into_inner(), &written[..500]);
        assert_eq!(unwritten.get_written(), &written[500..]);

        writeable = true;
        let socket = EOF2WB(Chunked(Cursor::new(Vec::with_capacity(1000)), 37));
        let mut connection_ctx = ConnectionContext {
            compression_threshold: -1,
            socket,
            unwritten_buf: unwritten,
            unread_buf: Default::default(),
            writeable
        };

        write(&mut connection_ctx).unwrap();

        let ConnectionContext { socket, unwritten_buf: unwritten, writeable, .. } = connection_ctx;

        assert!(writeable);
        assert!(unwritten.is_empty());
        assert_eq!(socket.0.0.into_inner(), &written[500..]);
    }

    #[test]
    fn full_write() {
        let mut socket = EOF2WB(Cursor::new([0u8; 0]));
        let mut buffer = Buffer::with_capacity(1000);
        let mut unwritten = Buffer::with_capacity(1000);

        let raw_buffer = buffer.get_unwritten(1000);
        rand::thread_rng().fill(raw_buffer);
        let written = unsafe { buffer.advance(1000) }.to_vec();

        let mut writeable = true;
        write_buffer(&mut socket, &mut buffer, &mut unwritten, &mut writeable).unwrap();

        assert!(!writeable);
        assert!(buffer.is_empty());
        assert_eq!(unwritten.get_written(), written);

        writeable = true;
        let socket = EOF2WB(Chunked(Cursor::new(Vec::with_capacity(1000)), 37));
        let mut connection_ctx = ConnectionContext {
            compression_threshold: -1,
            socket,
            unwritten_buf: unwritten,
            unread_buf: Default::default(),
            writeable
        };

        write(&mut connection_ctx).unwrap();

        let ConnectionContext { socket, unwritten_buf: unwritten, writeable, .. } = connection_ctx;

        assert!(writeable);
        assert!(unwritten.is_empty());
        assert_eq!(socket.0.0.into_inner(), written);
    }
}

mod read {
    use std::io::Cursor;
    use crate::{ConnectionContext, GlobalContext};
    use super::*;

    #[test]
    fn read_packets() {
        let stream = &[
            // One byte special case test
            0x01, 0xFF,
            // Packets with random data
            0x02, 0x05, 0x07,
            0x05, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x07, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x05, 0x01, 0x02, 0x03, 0x04, 0x05,
            // Fixed width varints
            0x81, 0x80, 0x00, 0xBB,
            0x85, 0x80, 0x00, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE
        ];
        let packets = &[
            &stream[1..2],
            &stream[3..5],
            &stream[6..11],
            &stream[12..19],
            &stream[20..25],
            &stream[28..29],
            &stream[32..37],
        ];

        let socket = EOF2WB(Chunked(Cursor::new(stream.to_vec()), 5));
        let mut global_ctx = GlobalContext::new();
        let mut connection_ctx = ConnectionContext::new(socket);
        let mut received_packets = Vec::new();

        read(&mut global_ctx, &mut connection_ctx, |packet, _, _| {
            received_packets.push(packet.0.to_vec());
            Ok(())
        }).unwrap();

        let ConnectionContext { unwritten_buf, unread_buf, writeable, .. } = connection_ctx;

        assert_eq!(&packets[..], &received_packets[..]);
        assert!(unwritten_buf.is_empty());
        assert!(unread_buf.is_empty());
        assert!(!writeable);
    }

    #[test]
    fn read_partial() {
        let mut buffer = [0x15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
        let (buffer1, buffer2) = buffer.split_at_mut(11);
        let socket = EOF2WB(Chunked(Cursor::new(&mut buffer1[..]), 5));

        let mut global_ctx = GlobalContext::new();
        let mut connection_ctx = ConnectionContext::new(socket);
        let mut received_packets = Vec::new();

        read(&mut global_ctx, &mut connection_ctx, |packet, _, _| {
            received_packets.push(packet.0.to_vec());
            Ok(())
        }).unwrap();

        let ConnectionContext { unwritten_buf, unread_buf, writeable, .. } = connection_ctx;

        assert!(received_packets.is_empty());
        assert!(unwritten_buf.is_empty());
        assert_eq!(unread_buf.len(), buffer1.len());
        assert!(!writeable);

        let socket = EOF2WB(Chunked(Cursor::new(&mut buffer2[..]), 5));
        let mut connection_ctx = ConnectionContext::new(socket);
        connection_ctx.unread_buf = unread_buf;

        read(&mut global_ctx, &mut connection_ctx, |packet, _, _| {
            received_packets.push(packet.0.to_vec());
            Ok(())
        }).unwrap();

        let ConnectionContext { unwritten_buf, unread_buf, writeable, .. } = connection_ctx;

        assert_eq!(&received_packets[..], &[&buffer[1..]]);
        assert!(unwritten_buf.is_empty());
        assert!(unread_buf.is_empty());
        assert!(!writeable);
    }
}
