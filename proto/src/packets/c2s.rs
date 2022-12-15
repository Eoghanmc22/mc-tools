pub mod handshake {
    use crate::{define_data, define_proto, primitive::VarInt, Direction};

    define_proto! {
        HandshakeState, 0, Direction::ClientToServer => {
           HandshakePacket<'a> = 0x00
        }
    }

    define_data! {
        pub struct HandshakePacket<'a> {
            pub protocol_version: u32 as VarInt,
            pub server_address: &'a str,
            pub server_port: u16,
            pub next_state: u8
        }
    }
}

pub mod login {
    use crate::{define_data, define_proto, primitive::Remaining, primitive::VarInt, Direction};

    define_proto! {
        LoginState, 2, Direction::ClientToServer => {
            LoginStartPacket<'a> = 0x00,
            EncryptionResponsePacket<'a> = 0x01,
            LoginPluginResponsePacket<'a> = 0x02
        }
    }

    define_data! {
        pub struct LoginStartPacket<'a> {
            pub username: &'a str,
            pub uuid: Option<u128>
        }
    }

    define_data! {
        pub struct EncryptionResponsePacket<'a> {
            pub shared_secret: &'a [u8],
            pub verify_token: &'a [u8]
        }
    }

    define_data! {
        pub struct LoginPluginResponsePacket<'a> {
            pub message_id: u32 as VarInt,
            pub successful: bool,
            pub data: &'a [u8] as Remaining<'a>
        }
    }
}

pub mod play {
    use crate::{define_data, define_proto, primitive::VarInt, Direction};

    define_proto! {
        PlayState, 3, Direction::ClientToServer => {
            TeleportConfirmPacket = 0x00,
            ChatMesssagePacket<'a> = 0x05,
            ClientSettingsPacket<'a> = 0x08,
            KeepAlivePacket = 0x12,
            PositionRotationPacket = 0x15,
            PlayerActionPacket = 0x1E,
            HeldSlotPacket = 0x28,
            AnimationPacket = 0x2F
        }
    }

    define_data! {
        pub struct TeleportConfirmPacket {
            pub id: u32 as VarInt
        }
    }

    define_data! {
        pub struct ChatMesssagePacket<'a> {
            pub message: &'a str,
            pub timestamp: u64,
            pub salt: u64,
            pub signature: &'a [u8],
            pub signed_preview: bool,
            pub seen_messages: Vec<SeenMessage<'a>>,
            pub last_seen: Option<SeenMessage<'a>>
        }
    }
    define_data! {
        pub struct SeenMessage<'a> {
            pub user: u128,
            pub signature: &'a [u8]
        }
    }

    define_data! {
        pub struct ClientSettingsPacket<'a> {
            pub locale: &'a str,
            pub view_distance: u8,
            pub chat_mode: u8, // TODO enums
            pub chat_colors: bool,
            pub skin_parts: u8,
            pub main_hand: u8, // TODO enums
            pub enable_text_filtering: bool,
            pub allow_server_listings: bool
        }
    }

    define_data! {
        pub struct KeepAlivePacket {
            pub id: u64
        }
    }

    define_data! {
        pub struct PositionRotationPacket {
            pub x: f64,
            pub y: f64,
            pub z: f64,

            pub yaw: f32,
            pub pitch: f32,

            pub on_ground: bool
        }
    }

    define_data! {
        pub struct PlayerActionPacket {
            pub entity_id: u32 as VarInt,
            pub action: u8, // TODO enums
            pub jump_boost: u32 as VarInt
        }
    }

    define_data! {
        pub struct HeldSlotPacket {
            pub slot: u16
        }
    }

    define_data! {
        pub struct AnimationPacket {
            pub hand: u8 // TODO enums
        }
    }
}
