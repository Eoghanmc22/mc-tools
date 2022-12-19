use std::{
    io::{Read, Write},
    sync::Arc,
};

use euclid::default::*;
use mc_io::{
    error::CommunicationError, ConnectionReadContext, ConnectionWriteContext, GlobalWriteContext,
    PacketHandler,
};
use proto::packets::c2s::play::PositionRotationPacket;

use crate::args::Args;

pub struct Player<S> {
    pub socket: Arc<S>,
    pub ctx_read: Option<ConnectionReadContext<Arc<S>>>,
    pub ctx_write: ConnectionWriteContext<Arc<S>>,

    pub entity_id: i32,
    pub proto_state: u8,
    pub username: String,

    // TODO Are 3 flags necessary?
    pub connected: bool,
    pub should_tick: bool,
    pub kicked: bool,

    pub position: Point3D<f64>,
    pub velocity: Vector3D<f64>,
}

impl<S> Player<S>
where
    for<'a> &'a S: Read + Write,
{
    pub fn new(stream: S, username: String) -> Self {
        let stream = Arc::new(stream);

        Self {
            socket: stream.clone(),
            ctx_read: Some(ConnectionReadContext::new(stream.clone())),
            ctx_write: ConnectionWriteContext::new(stream),
            entity_id: 0,
            proto_state: 0,
            username,
            connected: false,
            should_tick: false,
            kicked: false,
            position: Default::default(),
            velocity: Default::default(),
        }
    }

    pub fn tick(
        &mut self,
        args: &Args,
        ctx: &mut GlobalWriteContext,
    ) -> Result<(), CommunicationError> {
        if !self.should_tick || self.kicked {
            return Ok(());
        }

        self.position += self.velocity;

        if self.position.x.abs() > args.radius as f64 {
            self.velocity.x = -self.velocity.x;
        }
        if self.position.z.abs() > args.radius as f64 {
            self.velocity.z = -self.velocity.z;
        }

        // TODO: Send a variety of move packets
        let move_packet = PositionRotationPacket {
            x: self.position.x,
            y: self.position.y,
            z: self.position.z,
            on_ground: false,
            yaw: 0.0,
            pitch: 0.0,
        };
        self.ctx_write.write_packet(&move_packet, ctx)?;

        // TODO: Implement random actions

        Ok(())
    }
}

impl<S> PacketHandler for Player<S> {
    fn parse_and_handle(
        &mut self,
        packet: mc_io::RawPacket,
    ) -> Result<(), mc_io::error::ReadError> {
        todo!()
    }
}
