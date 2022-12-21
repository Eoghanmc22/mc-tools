// use helpers::{Chunked, EOF2WB};
//
// mod helpers {
//     use std::io::{self, ErrorKind, Read, Write};
//
//     pub struct EOF2WB<I: Read + Write>(pub I);
//
//     impl<I: Read + Write> Read for EOF2WB<I> {
//         fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//             match self.0.read(buf) {
//                 Ok(0) => Err(io::Error::new(ErrorKind::WouldBlock, "")),
//                 other => other,
//             }
//         }
//     }
//
//     impl<I: Read + Write> Write for EOF2WB<I> {
//         fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//             match self.0.write(buf) {
//                 Ok(0) => Err(io::Error::new(ErrorKind::WouldBlock, "")),
//                 other => other,
//             }
//         }
//
//         fn flush(&mut self) -> io::Result<()> {
//             self.0.flush()
//         }
//     }
//
//     pub struct Chunked<I: Read + Write>(pub I, pub usize);
//
//     impl<I: Read + Write> Read for Chunked<I> {
//         fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//             let len = usize::min(buf.len(), self.1);
//             self.0.read(&mut buf[..len])
//         }
//     }
//
//     impl<I: Read + Write> Write for Chunked<I> {
//         fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//             let len = usize::min(buf.len(), self.1);
//             self.0.write(&buf[..len])
//         }
//
//         fn flush(&mut self) -> io::Result<()> {
//             self.0.flush()
//         }
//     }
// }
//
// mod write {
//     use super::*;
//     use crate::{Buffer, ConnectionWriteContext, GlobalWriteContext};
//     use rand::Rng;
//     use std::{io::Cursor, rc::Rc};
//
//     #[test]
//     fn normal_write() {
//         let mut socket = Rc::new(EOF2WB(Chunked(Cursor::new(Vec::with_capacity(1000)), 37)));
//
//         let mut ctx_g = GlobalWriteContext::new();
//         let mut ctx_c = ConnectionWriteContext::new(socket);
//
//         let mut buffer = Buffer::with_capacity(1000);
//
//         let raw_buffer = buffer.get_unwritten(1000);
//         rand::thread_rng().fill(raw_buffer);
//         let written = unsafe { buffer.advance_write(1000) }.to_vec();
//
//         let mut writeable = true;
//         write_slice(
//             &mut socket,
//             buffer.get_written(),
//             &mut unwritten,
//             &mut writeable,
//         )
//         .unwrap();
//
//         assert!(writeable);
//         assert!(unwritten.is_empty());
//         assert_eq!(socket.0 .0.into_inner(), written);
//     }
//
//     #[test]
//     fn buffered_write() {
//         let mut socket = EOF2WB(Chunked(Cursor::new([0u8; 500]), 37));
//         let mut buffer = Buffer::with_capacity(1000);
//         let mut unwritten = Buffer::with_capacity(500);
//
//         let raw_buffer = buffer.get_unwritten(1000);
//         rand::thread_rng().fill(raw_buffer);
//         let written = unsafe { buffer.advance_write(1000) }.to_vec();
//
//         let mut writeable = true;
//         write_slice(
//             &mut socket,
//             buffer.get_written(),
//             &mut unwritten,
//             &mut writeable,
//         )
//         .unwrap();
//
//         assert!(!writeable);
//         assert_eq!(socket.0 .0.into_inner(), &written[..500]);
//         assert_eq!(unwritten.get_written(), &written[500..]);
//
//         let socket = EOF2WB(Chunked(Cursor::new(Vec::with_capacity(1000)), 37));
//         let mut connection_ctx = ConnectionWriteContext::new(socket);
//         connection_ctx.writeable = true;
//         connection_ctx.unwritten_buf = unwritten;
//
//         write(&mut connection_ctx).unwrap();
//
//         let ConnectionWriteContext {
//             socket,
//             unwritten_buf: unwritten,
//             writeable,
//             ..
//         } = connection_ctx;
//
//         assert!(writeable);
//         assert!(unwritten.is_empty());
//         assert_eq!(socket.0 .0.into_inner(), &written[500..]);
//     }
//
//     #[test]
//     fn full_write() {
//         let mut socket = EOF2WB(Cursor::new([0u8; 0]));
//         let mut buffer = Buffer::with_capacity(1000);
//         let mut unwritten = Buffer::with_capacity(1000);
//
//         let raw_buffer = buffer.get_unwritten(1000);
//         rand::thread_rng().fill(raw_buffer);
//         let written = unsafe { buffer.advance_write(1000) }.to_vec();
//
//         let mut writeable = true;
//         write_slice(
//             &mut socket,
//             buffer.get_written(),
//             &mut unwritten,
//             &mut writeable,
//         )
//         .unwrap();
//
//         assert!(!writeable);
//         assert!(buffer.is_empty());
//         assert_eq!(unwritten.get_written(), written);
//
//         let socket = EOF2WB(Chunked(Cursor::new(Vec::with_capacity(1000)), 37));
//         let mut connection_ctx = ConnectionWriteContext::new(socket);
//         connection_ctx.writeable = true;
//         connection_ctx.unwritten_buf = unwritten;
//
//         write(&mut connection_ctx).unwrap();
//
//         let ConnectionWriteContext {
//             socket,
//             unwritten_buf: unwritten,
//             writeable,
//             ..
//         } = connection_ctx;
//
//         assert!(writeable);
//         assert!(unwritten.is_empty());
//         assert_eq!(socket.0 .0.into_inner(), written);
//     }
// }
//
// mod read {
//     use super::*;
//     use crate::{CompressionReadContext, ConnectionReadContext, FramedPacket, GlobalReadContext};
//     use std::io::Cursor;
//
//     #[test]
//     fn read_packets() {
//         let stream = &[
//             // One byte special case test
//             0x01, 0xFF, // Packets with random data
//             0x02, 0x05, 0x07, 0x05, 0x01, 0x02, 0x03, 0x04, 0x05, 0x07, 0x01, 0x02, 0x03, 0x04,
//             0x05, 0x06, 0x07, 0x05, 0x01, 0x02, 0x03, 0x04, 0x05, // Fixed width varints
//             0x81, 0x80, 0x00, 0xBB, 0x85, 0x80, 0x00, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
//         ];
//         let packets = &[
//             &stream[1..2],
//             &stream[3..5],
//             &stream[6..11],
//             &stream[12..19],
//             &stream[20..25],
//             &stream[28..29],
//             &stream[32..37],
//         ];
//
//         let socket = EOF2WB(Chunked(Cursor::new(stream.to_vec()), 5));
//         let mut global_ctx = GlobalReadContext::new();
//         let mut connection_ctx = ConnectionReadContext::new(socket);
//         let mut received_packets = Vec::new();
//
//         read(
//             &mut global_ctx,
//             &mut connection_ctx,
//             |packet: &FramedPacket, _: CompressionReadContext| {
//                 received_packets.push(packet.0.to_vec());
//                 Ok(())
//             },
//         )
//         .unwrap();
//
//         let ConnectionReadContext { unread_buf, .. } = connection_ctx;
//
//         assert_eq!(&packets[..], &received_packets[..]);
//         assert!(unread_buf.is_empty());
//     }
//
//     #[test]
//     fn read_partial() {
//         let mut buffer = [
//             0x15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
//         ];
//         let (buffer1, buffer2) = buffer.split_at_mut(11);
//         let socket = EOF2WB(Chunked(Cursor::new(&mut buffer1[..]), 5));
//
//         let mut global_ctx = GlobalReadContext::new();
//         let mut connection_ctx = ConnectionReadContext::new(socket);
//         let mut received_packets = Vec::new();
//
//         read(
//             &mut global_ctx,
//             &mut connection_ctx,
//             |packet: &FramedPacket, _: CompressionReadContext| {
//                 received_packets.push(packet.0.to_vec());
//                 Ok(())
//             },
//         )
//         .unwrap();
//
//         let ConnectionReadContext { unread_buf, .. } = connection_ctx;
//
//         assert!(received_packets.is_empty());
//         assert_eq!(unread_buf.len(), buffer1.len());
//
//         let socket = EOF2WB(Chunked(Cursor::new(&mut buffer2[..]), 5));
//         let mut connection_ctx = ConnectionReadContext::new(socket);
//         connection_ctx.unread_buf = unread_buf;
//
//         read(
//             &mut global_ctx,
//             &mut connection_ctx,
//             |packet: &FramedPacket, _: CompressionReadContext| {
//                 received_packets.push(packet.0.to_vec());
//                 Ok(())
//             },
//         )
//         .unwrap();
//
//         let ConnectionReadContext { unread_buf, .. } = connection_ctx;
//
//         assert_eq!(&received_packets[..], &[&buffer[1..]]);
//         assert!(unread_buf.is_empty());
//     }
// }
