pub mod status {
    use crate::{define_data, define_proto, Direction};

    define_proto! {
        StatusProtoS2C, 1, Direction::ServerToClient => {
            StatusResponsePacket<'a> = 0x00,
            PingResponsePacket = 0x01
        }
    }

    define_data! {
        pub struct StatusResponsePacket<'a> {
            pub json: &'a str
        }
    }

    define_data! {
        pub struct PingResponsePacket {
            pub payload: u64
        }
    }
}

pub mod login {
    use crate::{define_data, define_proto, primitive::Remaining, primitive::VarInt, Direction};

    define_proto! {
        LoginProtoS2C, 2, Direction::ServerToClient => {
            DisconnectPacket<'a> = 0x00,
            EncryptionRequestPacket<'a> = 0x01,
            LoginSuccessPacket<'a> = 0x02,
            SetCompressionPacket = 0x03,
            LoginPluginRequestPacket<'a> = 0x04
        }
    }

    define_data! {
        pub struct DisconnectPacket<'a> {
            pub reason: &'a str
        }
    }

    define_data! {
        pub struct EncryptionRequestPacket<'a> {
            pub server_id: &'a str,
            pub public_key: &'a [u8],
            pub verify_token: &'a [u8]
        }
    }

    define_data! {
        pub struct LoginSuccessPacket<'a> {
            pub uuid: u128,
            pub username: &'a str,
            pub properties: Vec<Property<'a>>
        }
    }
    define_data! {
        pub struct Property<'a> {
            pub name: &'a str,
            pub value: &'a str,
            pub signature: Option<&'a str>
        }
    }

    define_data! {
        pub struct SetCompressionPacket {
            pub threshold: i32 as VarInt
        }
    }

    define_data! {
        pub struct LoginPluginRequestPacket<'a> {
            pub message_id: u32 as VarInt,
            pub channel: &'a str,
            pub data: Remaining<'a>
        }
    }
}

pub mod play {
    use crate::{define_data, define_proto, primitive::Remaining, primitive::VarInt, Direction};

    define_proto! {
        PlayProtoS2C, 3, Direction::ServerToClient => {
            DisconnectPacket<'a> = 0x19,
            KeepAlivePacket = 0x20,
            JoinGamePacket<'a> = 0x25,
            TeleportPacket = 0x39,
            TimePacket = 0x5C
        }
    }

    define_data! {
        pub struct DisconnectPacket<'a> {
            pub reason: &'a str
        }
    }

    define_data! {
        pub struct KeepAlivePacket {
            pub id: u64
        }
    }

    define_data! {
        pub struct JoinGamePacket<'a> {
            pub entity_id: u32,
            pub remaining: Remaining<'a> // Parsing is difficult. TODO NBT and enums
        }
    }

    define_data! {
        pub struct TeleportPacket {
            pub x: f64,
            pub y: f64,
            pub z: f64,

            pub yaw: f32,
            pub pitch: f32,

            pub flags: u8,

            pub id: u32 as VarInt,
            pub dismount: bool
        }
    }

    define_data! {
        pub struct TimePacket {
            pub world_age: u64,
            pub time_of_day: i64
        }
    }
}
