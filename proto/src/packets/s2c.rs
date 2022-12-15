pub mod login {
    use crate::{primitive::varint::var_int, Data, DecodingError};

    define_packet! {
        pub struct LoginStart<'a> {
            username: &'a str,
        }
    }

    define_packet! {
        pub struct LoginSuccess<'a> {
            uuid: u128,
            username: &'a str,
        }
    }

    define_packet! {
        pub struct SetCompression {
            threshold: u32,
        }
    }
}

pub mod play {}
