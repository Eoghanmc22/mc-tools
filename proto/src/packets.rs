pub mod handshake {
    use crate::{define_data, define_proto, primitive::VarInt, Direction};

    define_data! {
        pub struct Handshake<'a> {
            pub protocol_version: u32 as VarInt,
            pub server_address: &'a str,
            pub server_port: u16,
            pub next_state: u8
        }
    }

    define_proto!(
        HandshakeState, 0, Direction::ClientToServer => {
           Handshake<'a> = 0x00
        }
    );
}

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
