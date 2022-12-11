// TODO "Temporary" manual implemention of some packets

pub mod login {
    use crate::{Data, DecodingError};

    pub struct Handshake<'a> {
        protocol_version: u32,
        server_address: &'a str,
        server_port: u16,
        next_state: u32,
    }

    impl<'a> Data<'a> for Handshake<'a> {
        fn try_decode<'b: 'a>(_buffer: &'a mut &'b [u8]) -> Result<Handshake<'a>, DecodingError> {
            todo!()
        }

        fn expected_size(&self) -> usize {
            let mut len = 0;

            len += self.protocol_version.expected_size();
            len += self.server_address.expected_size();
            len += self.server_port.expected_size();
            len += self.next_state.expected_size();

            len
        }

        fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
            let buffer = self.protocol_version.encode(buffer);
            let buffer = self.server_address.encode(buffer);
            let buffer = self.server_port.encode(buffer);
            let buffer = self.next_state.encode(buffer);
            buffer
        }
    }
}
