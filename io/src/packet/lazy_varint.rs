use std::mem;
use binary::varint;

pub struct LazyVarint<'a> {
    buffer: &'a mut [u8]
}

impl LazyVarint {
     pub fn write(self, num: i32) {
         let available = self.buffer.len();
         let (raw_bytes, len) = varint::encode::i32_raw(num);
         assert!(available >= len, "Lazy varint buffer is too small");

         self.buffer[..len].copy_from_slice(&raw_bytes[..len]);

         if available > len {
             for byte in len..available - 1 {
                 self.buffer[byte] = 0b10000000;
             }
         }
     }
}

pub fn lazy_varint<'a>(buffer: &'a mut &'a mut [u8], max_width: usize) -> LazyVarint<'a> {
    let (varint, remaining) = (*buffer).split_at_mut(max_width);
    *buffer = remaining;

    LazyVarint {
        buffer: varint
    }
}
