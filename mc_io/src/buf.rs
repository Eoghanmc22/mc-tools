// Taken from graphite and modified for use case

use std::ptr;

#[derive(Clone, Debug)]
pub struct Buffer {
    vec: Vec<u8>,
    write_index: usize,
    read_index: usize,
}

impl Buffer {
    pub const fn new() -> Self {
        Self {
            vec: Vec::new(),
            write_index: 0,
            read_index: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity),
            ..Self::new()
        }
    }

    pub fn len(&self) -> usize {
        self.write_index - self.read_index
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn reset(&mut self) {
        self.write_index = 0;
        self.read_index = 0;
    }

    pub fn into_written(mut self) -> Vec<u8> {
        unsafe {
            self.vec.set_len(self.write_index);
        }
        self.vec.drain(..self.read_index);
        self.vec
    }

    pub fn get_written(&self) -> &[u8] {
        let ptr = self.vec.as_ptr();
        unsafe { std::slice::from_raw_parts(ptr.add(self.read_index), self.len()) }
    }

    pub fn get_unwritten(&mut self, capacity: usize) -> &mut [u8] {
        self.vec.reserve(self.write_index + capacity);

        unsafe {
            let ptr = self.vec.as_mut_ptr().add(self.write_index);
            std::slice::from_raw_parts_mut(ptr, capacity)
        }
    }

    pub fn copy_from(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        self.get_unwritten(bytes.len()).copy_from_slice(bytes);
        unsafe {
            self.advance_write(bytes.len());
        }
    }

    pub fn consume(&mut self, amount: usize) {
        debug_assert!(
            amount <= self.len(),
            "amount {} must be <= the length {}",
            amount,
            self.len()
        );

        let remaining = self.len() - amount;

        unsafe {
            let src = self.vec.as_ptr().add(self.read_index + amount);
            let dst = self.vec.as_mut_ptr();
            ptr::copy(src, dst, remaining);
        }

        self.write_index = remaining;
        self.read_index = 0
    }

    /// This function should be used after successfully writing some data with `get_unwritten`
    ///
    /// # Safety
    /// 1. `advance` must be less than the capacity requested in `get_unwritten`
    /// 2.  At least `advance` bytes must have been written to the slice returned by `get_unwritten`,
    ///     otherwise `get_written` will return uninitialized memory
    pub unsafe fn advance_write(&mut self, advance: usize) -> &[u8] {
        debug_assert!(
            self.write_index + advance <= self.vec.capacity(),
            "advance {} must be <= the remaining bytes {}",
            advance,
            self.vec.capacity() - self.write_index
        );

        let ptr = self.vec.as_ptr().add(self.write_index);
        let slice = std::slice::from_raw_parts(ptr, advance);

        self.write_index += advance;

        slice
    }

    pub fn advance_read(&mut self, advance: usize) -> &[u8] {
        assert!(
            self.read_index + advance <= self.write_index,
            "Can not advance read idx ({}) past write ({}), A: {}",
            self.read_index,
            self.write_index,
            advance
        );

        let slice = unsafe {
            let ptr = self.vec.as_ptr().add(self.read_index);
            std::slice::from_raw_parts(ptr, advance)
        };

        self.read_index += advance;

        slice
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_from() {
        let mut buffer = Buffer::new();

        let copy1 = &[0, 1, 2, 3, 4, 5];
        buffer.copy_from(copy1);

        let copy2 = &[6, 7, 8];
        buffer.copy_from(copy2);

        let mut expected = Vec::new();
        expected.extend_from_slice(copy1);
        expected.extend_from_slice(copy2);

        assert_eq!(buffer.into_written(), expected);
    }

    #[test]
    fn consume() {
        let mut buffer = Buffer::new();

        let data = [5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7];
        buffer.copy_from(&data);

        let consumed = 5;
        buffer.consume(consumed);

        assert_eq!(buffer.into_written(), data[consumed..]);
    }

    #[test]
    fn advance() {
        let mut buffer = Buffer::new();

        let raw_buffer = buffer.get_unwritten(5);
        raw_buffer.copy_from_slice(&[10; 5]);

        let advanced = unsafe { buffer.advance_write(5) };

        assert_eq!(advanced, &[10; 5]);
        assert_eq!(buffer.into_written(), &[10; 5]);
    }
}
