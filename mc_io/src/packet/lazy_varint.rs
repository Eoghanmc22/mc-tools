use std::mem;

use proto::{primitive::VarNum, Data};

pub struct LazyVarint<'a, const MAX_WIDTH: usize> {
    buffer: &'a mut [u8; MAX_WIDTH],
}

impl<'a, const MAX_WIDTH: usize> LazyVarint<'a, MAX_WIDTH> {
    pub fn new(buffer: &mut &'a mut [u8]) -> Self {
        // Lifetime dance taken from `impl Write for &mut [u8]`.
        let (a, b) = mem::take(buffer).split_array_mut();
        *buffer = b;

        Self { buffer: a }
    }

    pub fn write(self, num: impl Into<VarNum<MAX_WIDTH>>) {
        // TODO this fails silently if the varint doesnt fit
        let remaining = num.into().encode(&mut *self.buffer);
        let remaining = remaining.len();

        if remaining > 0 {
            self.buffer[MAX_WIDTH - remaining - 1] |= 0b1000_0000;
            for i in 0..remaining {
                let idx = MAX_WIDTH - i - 1;
                self.buffer[idx] = 0b1000_0000;
            }
            self.buffer[MAX_WIDTH - 1] &= 0b0111_1111;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Buffer;
    use std::mem;

    #[test]
    fn varint_roundtrip() {
        do_varint_roundtrip::<3>(0);
        do_varint_roundtrip::<5>(0);
        do_varint_roundtrip::<3>(100);
        do_varint_roundtrip::<5>(100);
        do_varint_roundtrip::<3>(100000);
        do_varint_roundtrip::<5>(100000);
        do_varint_roundtrip::<5>(10000000);
        do_varint_roundtrip::<5>(-100);
        do_varint_roundtrip::<5>(-100000);
        do_varint_roundtrip::<5>(-10000000);

        do_varint_roundtrip::<3>(0b0001_1111__1111_1111__1111_1111);
        do_varint_roundtrip::<3>(0b0001_0000__0000_0000__0000_0000);
    }

    fn do_varint_roundtrip<const MAX_WIDTH: usize>(num: i32) {
        let mut buffer = Buffer::with_capacity(1 + MAX_WIDTH + 1);
        let mut raw_buffer = buffer.get_unwritten(1 + MAX_WIDTH + 1);

        write_byte(&mut raw_buffer, 0xFF);
        let varint = LazyVarint::<MAX_WIDTH>::new(&mut raw_buffer);
        write_byte(&mut raw_buffer, 0xFF);
        varint.write(num);

        let mut raw_buffer = unsafe { buffer.advance(1 + MAX_WIDTH + 1) };

        assert_eq!(read_byte(&mut raw_buffer), 0xFF);
        assert_eq!(
            num,
            VarNum::<MAX_WIDTH>::try_decode(&mut raw_buffer)
                .unwrap()
                .into()
        );
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
