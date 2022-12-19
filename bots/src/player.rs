use std::{
    io::{Read, Write},
    sync::Arc,
};

use euclid::default::*;
use mc_io::{
    error::{CommunicationError, ReadError},
    ConnectionReadContext, ConnectionWriteContext, GlobalWriteContext, PacketHandler, RawPacket,
};
use proto::packets::{
    c2s::{self, play::PositionRotationPacket},
    s2c::{
        login::{self, LoginProtoS2C, PacketHandlerLoginProtoS2C},
        play::{self, PacketHandlerPlayProtoS2C, PlayProtoS2C},
    },
};

use crate::args::Args;

pub struct Player<S> {
    pub socket: Arc<S>,
    pub ctx_read: Option<ConnectionReadContext<Arc<S>>>,
    pub ctx_write: ConnectionWriteContext<Arc<S>>,

    // TODO This is bad
    pub g_ctx_write: Option<GlobalWriteContext>,

    pub entity_id: u32,
    pub proto_state: u8,
    pub username: String,
    pub uuid: u128,

    // TODO Are 3 flags necessary?
    pub connected: bool,
    pub should_tick: bool,
    pub kicked: bool,
    // TODO remove
    pub compression_threshold_dirty: Option<i32>,

    pub position: Point3D<f64>,
    pub velocity: Vector3D<f64>,

    state: u8,
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
            state: LoginProtoS2C::PROTOCOL_ID,
            uuid: 0,
            compression_threshold_dirty: None,
            g_ctx_write: None,
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

impl<S> PacketHandler for Player<S>
where
    for<'a> &'a S: Write,
{
    fn parse_and_handle(&mut self, packet: RawPacket) -> Result<(), CommunicationError> {
        // TODO Fix name
        match self.state {
            LoginProtoS2C::PROTOCOL_ID => self.parse_and_handle_login_proto_s2_c(packet),
            PlayProtoS2C::PROTOCOL_ID => self.parse_and_handle_play_proto_s2_c(packet),
            _ => Err(ReadError::BadProtocolState.into()),
        }
    }
}

impl<S> PacketHandlerLoginProtoS2C for Player<S>
where
    for<'a> &'a S: Write,
{
    type Error = CommunicationError;

    fn handle_disconnect_packet(
        &mut self,
        packet: login::DisconnectPacket,
    ) -> Result<(), Self::Error> {
        Err(CommunicationError::Kicked(packet.reason.to_owned()))
    }

    fn handle_encryption_request_packet(
        &mut self,
        _: login::EncryptionRequestPacket,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn handle_login_success_packet(
        &mut self,
        packet: login::LoginSuccessPacket,
    ) -> Result<(), Self::Error> {
        self.uuid = packet.uuid;
        self.username = packet.username.to_owned();
        self.state = PlayProtoS2C::PROTOCOL_ID;

        Ok(())
    }

    fn handle_set_compression_packet(
        &mut self,
        packet: login::SetCompressionPacket,
    ) -> Result<(), Self::Error> {
        self.compression_threshold_dirty = Some(packet.threshold);

        Ok(())
    }

    fn handle_login_plugin_request_packet(
        &mut self,
        _: login::LoginPluginRequestPacket,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

impl<S> PacketHandlerPlayProtoS2C for Player<S>
where
    for<'a> &'a S: Write,
{
    type Error = CommunicationError;

    fn handle_disconnect_packet(
        &mut self,
        packet: play::DisconnectPacket,
    ) -> Result<(), Self::Error> {
        Err(CommunicationError::Kicked(packet.reason.to_owned()))
    }

    fn handle_keep_alive_packet(
        &mut self,
        packet: play::KeepAlivePacket,
    ) -> Result<(), Self::Error> {
        if let Some(ref mut ctx) = self.g_ctx_write {
            self.ctx_write
                .write_packet(&c2s::play::KeepAlivePacket { id: packet.id }, ctx)?;
        }

        Ok(())
    }

    fn handle_join_game_packet(&mut self, packet: play::JoinGamePacket) -> Result<(), Self::Error> {
        self.entity_id = packet.entity_id;

        if let Some(ref mut ctx) = self.g_ctx_write {
            self.ctx_write.write_packet(
                &c2s::play::ClientSettingsPacket {
                    locale: "en_US",
                    view_distance: 10,
                    chat_mode: 0,
                    chat_colors: true,
                    skin_parts: 0x7F,
                    main_hand: 0,
                    enable_text_filtering: false,
                    allow_server_listings: true,
                },
                ctx,
            )?;
        }

        Ok(())
    }

    fn handle_teleport_packet(&mut self, packet: play::TeleportPacket) -> Result<(), Self::Error> {
        if packet.flags & 0b10000 == 0 {
            self.position.x = packet.x;
        } else {
            self.position.x += packet.x;
        }
        if packet.flags & 0b01000 == 0 {
            self.position.y = packet.y;
        } else {
            self.position.y += packet.y;
        }
        if packet.flags & 0b00100 == 0 {
            self.position.z = packet.z;
        } else {
            self.position.z += packet.z;
        }

        if let Some(ref mut ctx) = self.g_ctx_write {
            self.ctx_write
                .write_packet(&c2s::play::TeleportConfirmPacket { id: packet.id }, ctx)?;
        }

        Ok(())
    }
}
