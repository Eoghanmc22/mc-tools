use crate::{Data, DecodingError};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = !SEGMENT_BITS;
const REMAINING_MASK: u64 = !(SEGMENT_BITS as u64);

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct VarNum<const WIDTH: usize>(pub u64);

// TODO fast impl
impl<'a, const WIDTH: usize> Data<'a> for VarNum<WIDTH> {
    fn try_decode(buffer: &mut &'a [u8]) -> Result<Self, DecodingError> {
        let mut val = 0;

        for position in 0..WIDTH {
            let next_byte = u8::try_decode(buffer)?;
            let offset = position * 7;

            val |= ((next_byte & SEGMENT_BITS) as u64) << offset;

            if next_byte & CONTINUE_BIT == 0 {
                return Ok(Self(val));
            }
        }

        Err(DecodingError::BadData)
    }

    fn expected_size(&self) -> usize {
        // TODO actual size
        WIDTH
    }

    fn encode<'b>(&self, mut buffer: &'b mut [u8]) -> &'b mut [u8] {
        let val = self.0;

        for position in 0..WIDTH {
            let val = val >> position * 7;

            let byte = val as u8 & SEGMENT_BITS;
            let continue_bit = if val & REMAINING_MASK != 0 {
                CONTINUE_BIT
            } else {
                0
            };

            buffer = (byte | continue_bit).encode(buffer);

            if continue_bit == 0 {
                break;
            }
        }

        buffer
    }
}

pub type V21 = VarNum<3>;
pub type VarInt = VarNum<5>;
pub type VarLong = VarNum<10>;

pub fn v21(num: u32) -> V21 {
    VarNum(num as u64)
}
pub fn var_int(num: i32) -> VarInt {
    VarNum(num as u32 as u64)
}
pub fn var_long(num: i64) -> VarLong {
    VarNum(num as u64)
}

macro_rules! convert_impl {
    ($other: ty, $intermediate: ty) => {
        impl<const WIDTH: usize> From<$other> for VarNum<WIDTH> {
            fn from(other: $other) -> Self {
                Self(other as $intermediate as u64)
            }
        }

        impl<const WIDTH: usize> From<VarNum<WIDTH>> for $other {
            fn from(var_num: VarNum<WIDTH>) -> Self {
                var_num.0 as $other
            }
        }
    };
}

convert_impl!(u32, u32);
convert_impl!(i32, u32);
convert_impl!(u64, u64);
convert_impl!(i64, u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip() {
        do_varint_roundtrip(v21(0));
        do_varint_roundtrip(var_int(0));
        do_varint_roundtrip(var_long(0));

        do_varint_roundtrip(v21(100));
        do_varint_roundtrip(var_int(100));
        do_varint_roundtrip(var_long(100));

        do_varint_roundtrip(v21(100000));
        do_varint_roundtrip(var_int(100000));
        do_varint_roundtrip(var_long(100000));

        do_varint_roundtrip(var_int(10000000));
        do_varint_roundtrip(var_long(10000000));

        do_varint_roundtrip(var_long(10000000000000));

        do_varint_roundtrip(var_int(-100));
        do_varint_roundtrip(var_long(-100));

        do_varint_roundtrip(var_int(-100000));
        do_varint_roundtrip(var_long(-100000));

        do_varint_roundtrip(var_int(-10000000));
        do_varint_roundtrip(var_long(-10000000));

        do_varint_roundtrip(var_long(-10000000000000));
    }

    fn do_varint_roundtrip<const WIDTH: usize>(num: VarNum<WIDTH>) {
        let mut buffer = [0; WIDTH];

        let expected_size = num.expected_size();
        let remaining = num.encode(&mut buffer[..expected_size]);
        let used = expected_size - remaining.len();

        let read = VarNum::<WIDTH>::try_decode(&mut &buffer[..used]).unwrap();

        assert_eq!(
            read,
            num,
            "w: {WIDTH}, e: {expected_size}, u: {used} b: {:?}",
            &buffer[..used]
        );
    }
}
