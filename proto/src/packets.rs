// TODO "Temporary" manual implemention of some packets
// TODO Packet trait

pub mod handshake {
    use crate::{primitive::varint::var_int, Data, DecodingError};

    pub struct Handshake<'a> {
        protocol_version: u32,
        server_address: &'a str,
        server_port: u16,
        next_state: u32,
    }

    impl<'a> Data<'a> for Handshake<'a> {
        fn try_decode<'b: 'a>(_buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
            todo!()
        }

        fn expected_size(&self) -> usize {
            let mut len = 1;

            len += self.protocol_version.expected_size();
            len += self.server_address.expected_size();
            len += self.server_port.expected_size();
            len += self.next_state.expected_size();

            len
        }

        fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
            let buffer = var_int(0x00).encode(buffer);
            let buffer = self.protocol_version.encode(buffer);
            let buffer = self.server_address.encode(buffer);
            let buffer = self.server_port.encode(buffer);
            let buffer = self.next_state.encode(buffer);
            buffer
        }
    }
}

pub mod login {
    use crate::{primitive::varint::var_int, Data, DecodingError};

    pub struct LoginStart<'a> {
        username: &'a str,
    }

    impl<'a> Data<'a> for LoginStart<'a> {
        fn try_decode<'b: 'a>(_buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
            todo!()
        }

        fn expected_size(&self) -> usize {
            let mut len = 1;

            len += self.username.expected_size();

            len
        }

        fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
            let buffer = var_int(0x00).encode(buffer);
            let buffer = self.username.encode(buffer);
            buffer
        }
    }

    pub struct LoginSuccess<'a> {
        uuid: u128,
        username: &'a str,
    }

    impl<'a> Data<'a> for LoginSuccess<'a> {
        fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
            Ok(LoginSuccess {
                uuid: Data::try_decode(buffer)?,
                username: Data::try_decode(buffer)?,
            })
        }

        fn expected_size(&self) -> usize {
            todo!()
        }

        fn encode<'b>(&self, _buffer: &'b mut [u8]) -> &'b mut [u8] {
            todo!()
        }
    }

    pub struct SetCompression {
        threshold: u32,
    }

    impl<'a> Data<'a> for SetCompression {
        fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
            Ok(SetCompression {
                threshold: Data::try_decode(buffer)?,
            })
        }

        fn expected_size(&self) -> usize {
            todo!()
        }

        fn encode<'b>(&self, _buffer: &'b mut [u8]) -> &'b mut [u8] {
            todo!()
        }
    }
}

pub mod play {}
