use crate::{error::CommunicationError, CompressionReadContext, FramedPacket, RawPacket};

use super::helpers;

pub trait PacketHandler<C> {
    fn parse_and_handle(
        &mut self,
        packet: RawPacket,
        ctx: &mut C,
    ) -> Result<(), CommunicationError>;
    fn compression_threshold(&self) -> i32;
}

pub(crate) fn create_handler<'a, 'b: 'a, C: 'b, H: PacketHandler<C>>(
    handler: &'a mut H,
    ctx: &'b mut C,
) -> impl FnMut(&FramedPacket, CompressionReadContext) -> Result<(), CommunicationError> + 'a {
    move |packet, read_ctx| {
        let packet = helpers::read_packet(packet, read_ctx, handler.compression_threshold())?;
        handler.parse_and_handle(packet, ctx)
    }
}
