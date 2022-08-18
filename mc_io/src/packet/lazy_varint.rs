use binary::varint;

pub struct LazyVarint<'a> {
    buffer: &'a mut [u8],
}

impl<'a> LazyVarint<'a> {
    pub fn new(buffer: &'a mut [u8], max_width: usize) -> (Self, &'a mut [u8]) {
        let (varint, remaining) = buffer.split_at_mut(max_width);

        (Self { buffer: varint }, remaining)
    }

    pub fn write(self, num: i32) {
        let available = self.buffer.len();
        let (raw_bytes, len) = varint::encode::i32_raw(num);
        assert!(available >= len, "Lazy varint buffer is too small");

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
