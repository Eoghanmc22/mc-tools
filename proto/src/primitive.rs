pub mod varint;

use crate::{Data, DecodingError};
use std::mem;

use self::varint::{var_int, VarInt};

macro_rules! impl_data_primitive {
    ($ty:ty, $bits:expr) => {
        impl<'a> Data<'a> for $ty {
            fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
                if buffer.len() < mem::size_of::<Self>() {
                    return Err(DecodingError::EOF);
                }

                let (num, remaining) = buffer.split_array_ref();
                *buffer = remaining;

                Ok(Self::from_be_bytes(*num))
            }

            fn expected_size(&self) -> usize {
                mem::size_of::<Self>()
            }

            fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
                let (num, remaining) = buffer.split_array_mut();
                *num = self.to_be_bytes();

                remaining
            }
        }
    };
    ($ty:ty) => {
        impl_data_primitive!($ty, <$ty>::BITS as usize);
    };
}

impl_data_primitive!(u8);
impl_data_primitive!(i8);
impl_data_primitive!(u16);
impl_data_primitive!(i16);
impl_data_primitive!(u32);
impl_data_primitive!(i32);
impl_data_primitive!(u64);
impl_data_primitive!(i64);
impl_data_primitive!(u128);
impl_data_primitive!(f32, 32);
impl_data_primitive!(f64, 64);

// String
// Chat
// Identifier
// varint
// varlong
// entity meta
// slot
// nbt tag
// position
// angle
// arrays
// enums

impl<'a> Data<'a> for bool {
    fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
        let byte = u8::try_decode(buffer)?;
        Ok(byte != 0)
    }

    fn expected_size(&self) -> usize {
        1
    }

    fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
        (*self as u8).encode(buffer)
    }
}

impl<'a, D> Data<'a> for Option<D>
where
    D: Data<'a>,
{
    fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
        let present = bool::try_decode(buffer)?;
        if present {
            Ok(Some(D::try_decode(buffer)?))
        } else {
            Ok(None)
        }
    }

    fn expected_size(&self) -> usize {
        match self {
            Some(inner) => 1 + inner.expected_size(),
            None => 1,
        }
    }

    fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
        match self {
            Some(inner) => {
                let buffer = true.encode(buffer);
                inner.encode(buffer)
            }
            None => false.encode(buffer),
        }
    }
}

impl<'a> Data<'a> for &'a [u8] {
    fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
        let len = VarInt::try_decode(buffer)?.0 as usize;

        if buffer.len() < len {
            return Err(DecodingError::EOF);
        }

        let (data, remaining) = buffer.split_at(len);
        *buffer = remaining;

        Ok(data)
    }

    fn expected_size(&self) -> usize {
        var_int(self.len() as i32).expected_size() + self.len()
    }

    fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
        let len = var_int(self.len() as i32);

        let buffer = len.encode(buffer);
        buffer[..self.len()].copy_from_slice(self);
        &mut buffer[self.len()..]
    }
}

impl<'a> Data<'a> for &'a str {
    fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
        let data = <&[u8]>::try_decode(buffer)?;
        core::str::from_utf8(data).map_err(|_| DecodingError::BadData)
    }

    fn expected_size(&self) -> usize {
        self.as_bytes().expected_size()
    }

    fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
        self.as_bytes().encode(buffer)
    }
}
