use binary::varint;
use std::mem;

pub struct LazyVarint<'a> {
    buffer: &'a mut [u8],
}

impl<'a> LazyVarint<'a> {
    pub fn new(buffer: &mut &'a mut [u8], max_width: usize) -> Self {
        // Lifetime dance taken from `impl Write for &mut [u8]`.
        let (a, b) = mem::take(buffer).split_at_mut(max_width);
        *buffer = b;

        Self { buffer: a }
    }

    pub fn write(self, num: i32) {
        let available = self.buffer.len();
        let (raw_bytes, len) = varint::encode::i32_raw(num);

        debug_assert!(
            available >= len,
            "Lazy varint buffer is too small, available: {available}, len: {len}, num: {num}"
        );

        self.buffer[..len].copy_from_slice(&raw_bytes[..len]);

        if available > len {
            self.buffer[len - 1] |= 0b10000000;
            for byte in len..available - 1 {
                self.buffer[byte] = 0b10000000;
            }
            self.buffer[available - 1] = 0b00000000;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Buffer;
    use binary::slice_serialization::{SliceSerializable, VarInt};
    use std::mem;

    #[test]
    fn varint_roundtrip() {
        do_varint_roundtrip(0, 3);
        do_varint_roundtrip(0, 5);
        do_varint_roundtrip(100, 3);
        do_varint_roundtrip(100, 5);
        do_varint_roundtrip(100000, 3);
        do_varint_roundtrip(100000, 5);
        do_varint_roundtrip(10000000, 5);
        do_varint_roundtrip(-100, 5);
        do_varint_roundtrip(-100000, 5);
        do_varint_roundtrip(-10000000, 5);
    }

    fn do_varint_roundtrip(num: i32, max_width: usize) {
        let mut buffer = Buffer::with_capacity(1 + max_width + 1);
        let mut raw_buffer = buffer.get_unwritten(1 + max_width + 1);

        write_byte(&mut raw_buffer, 0xFF);
        let varint = LazyVarint::new(&mut raw_buffer, max_width);
        write_byte(&mut raw_buffer, 0xFF);
        varint.write(num);

        let mut raw_buffer = unsafe { buffer.advance(1 + max_width + 1) };

        assert_eq!(read_byte(&mut raw_buffer), 0xFF);
        assert_eq!(VarInt::read(&mut raw_buffer).unwrap(), num);
        assert_eq!(read_byte(&mut raw_buffer), 0xFF);
    }

    fn write_byte(buffer: &mut &mut [u8], byte: u8) {
        buffer[0] = byte;
        // Lifetime dance taken from `impl Write for &mut [u8]`.
        let (_, b) = mem::take(buffer).split_at_mut(1);
        *buffer = b;
    }

    fn read_byte(buffer: &mut &[u8]) -> u8 {
        let byte = buffer[0];
        *buffer = &buffer[1..];
        byte
    }
}
