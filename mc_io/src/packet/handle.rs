use crate::{error::CommunicationError, CompressionReadContext, FramedPacket, RawPacket};

use super::helpers;

pub trait PacketHandler {
    fn parse_and_handle(&mut self, packet: RawPacket) -> Result<(), CommunicationError>;
    fn compression_threshold(&self) -> i32;
}

pub(crate) fn create_handler<'a, H: PacketHandler>(
    handler: &'a mut H,
) -> impl FnMut(&FramedPacket, CompressionReadContext) -> Result<(), CommunicationError> + 'a {
    |packet, ctx| {
        let packet = helpers::read_packet(packet, ctx, handler.compression_threshold())?;
        handler.parse_and_handle(packet)
    }
}
