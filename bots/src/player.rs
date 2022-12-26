use std::{
    io::{Read, Write},
    ops::Mul,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use euclid::{default::*, Angle};
use mc_io::{
    error::{CommunicationError, ReadError},
    ConnectionReadContext, ConnectionWriteContext, PacketHandler, RawPacket,
};
use proto::packets::{
    c2s::{
        self,
        play::{
            AnimationPacket, ChatMesssagePacket, HeldSlotPacket, PlayerActionPacket,
            PositionPacket, PositionRotationPacket,
        },
    },
    s2c::{
        login::{self, LoginProtoS2C, PacketHandlerLoginProtoS2C},
        play::{self, PacketHandlerPlayProtoS2C, PlayProtoS2C},
    },
};
use rand::{distributions::Standard, prelude::Distribution, seq::SliceRandom, Rng};

use crate::{
    args::{Args, Movement},
    context::Context,
};

pub struct Player<S> {
    pub socket: Arc<S>,
    pub ctx_read: Option<ConnectionReadContext<Arc<S>>>,
    pub ctx_write: Option<ConnectionWriteContext<Arc<S>>>,

    pub entity_id: u32,
    pub proto_state: u8,
    pub username: String,
    pub uuid: u128,

    // TODO Are 3 flags necessary?
    pub connected: bool,
    pub should_tick: bool,
    pub kicked: bool,

    pub compression_threshold: i32,

    pub position: Point3D<f64>,
    pub velocity: Vector3D<f64>,
    pub angle_bias: Rotation3D<f64>,

    pub last_game_time: (u64, Instant),
    pub tps: f64,

    pub join_time: Option<Instant>,

    pub sneaking: bool,
    pub sprinting: bool,

    state: u8,
}

impl<S> Player<S>
where
    for<'a> &'a S: Read + Write,
{
    pub fn new(stream: S, username: String) -> Self {
        let stream = Arc::new(stream);
        let velocity = (rand::random(), rand::random());

        Self {
            socket: stream.clone(),
            ctx_read: Some(ConnectionReadContext::new(stream.clone())),
            ctx_write: Some(ConnectionWriteContext::new(stream)),
            entity_id: 0,
            proto_state: 0,
            username,
            connected: false,
            should_tick: false,
            kicked: false,
            position: Default::default(),
            velocity: Vector3D::new(velocity.0, 0.0, velocity.1)
                .normalize()
                .mul(0.2),
            angle_bias: Rotation3D::around_y(Angle::degrees(rand::random::<f64>() * 20.0 - 10.0)),
            state: LoginProtoS2C::PROTOCOL_ID,
            uuid: 0,
            compression_threshold: -1,
            last_game_time: (0, Instant::now()),
            tps: f64::NAN,
            join_time: None,
            sneaking: false,
            sprinting: false,
        }
    }

    pub fn tick(&mut self, args: &Args, ctx: &mut Context) -> Result<(), CommunicationError> {
        if !self.should_tick || self.kicked {
            return Ok(());
        }

        self.ctx_write
            .as_mut()
            .ok_or("Connection writer theft")?
            .write_packets(&mut ctx.g_write_ctx, self.compression_threshold, |writer| {
                if !args.no_move {
                    match args.movement {
                        Movement::Biased => {
                            self.velocity = self.angle_bias.transform_vector3d(self.velocity);
                        }
                        Movement::Consistant => {
                            // No change to velocity
                        }
                        Movement::Random => {
                            let velocity = (rand::random(), rand::random());
                            self.velocity = Vector3D::new(velocity.0, 0.0, velocity.1)
                                .normalize()
                                .mul(0.2);
                        }
                    }
                    self.position += self.velocity;

                    if self.position.x.abs() > args.radius as f64 {
                        self.velocity.x = -self.velocity.x;
                    }
                    if self.position.z.abs() > args.radius as f64 {
                        self.velocity.z = -self.velocity.z;
                    }

                    if !args.no_yaw {
                        let yaw = self
                            .velocity
                            .xz()
                            .angle_to(Vector2D::new(0.0, 1.0))
                            .to_f32()
                            .to_degrees();

                        let move_packet = PositionRotationPacket {
                            x: self.position.x,
                            y: self.position.y,
                            z: self.position.z,
                            yaw,
                            pitch: 0.0,
                            on_ground: false,
                        };

                        writer.write_packet(&move_packet)?;
                    } else {
                        let move_packet = PositionPacket {
                            x: self.position.x,
                            y: self.position.y,
                            z: self.position.z,
                            on_ground: false,
                        };

                        writer.write_packet(&move_packet)?;
                    }
                }

                if !args.no_action && rand::random::<f64>() < args.action_chance {
                    let action: Action = rand::random();
                    match action {
                        Action::SendChat => {
                            let message = ctx
                                .messages
                                .choose(&mut rand::thread_rng())
                                .map(|it| it.as_str())
                                .unwrap_or("Chat message");

                            let packet = ChatMesssagePacket {
                                message,
                                timestamp: SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                                salt: 0,
                                signature: &[],
                                signed_preview: false,
                                seen_messages: Vec::new(),
                                last_seen: None,
                            };

                            writer.write_packet(&packet)?;
                        }
                        Action::Punch => {
                            let packet = AnimationPacket {
                                hand: rand::thread_rng().gen_range(0..2),
                            };

                            writer.write_packet(&packet)?;
                        }
                        Action::ToggleSneak => {
                            self.sneaking = !self.sneaking;

                            let packet = PlayerActionPacket {
                                entity_id: self.entity_id,
                                action: if self.sneaking { 0 } else { 1 },
                                jump_boost: 0,
                            };

                            writer.write_packet(&packet)?;
                        }
                        Action::ToggleSprint => {
                            self.sprinting = !self.sprinting;

                            let packet = PlayerActionPacket {
                                entity_id: self.entity_id,
                                action: if self.sprinting { 3 } else { 4 },
                                jump_boost: 0,
                            };

                            writer.write_packet(&packet)?;
                        }
                        Action::HeldItem => {
                            let packet = HeldSlotPacket {
                                slot: rand::thread_rng().gen_range(0..9),
                            };

                            writer.write_packet(&packet)?;
                        }
                    }
                }

                Ok(())
            })
    }
}

impl<S> PacketHandler<Context> for Player<S>
where
    for<'a> &'a S: Write,
{
    fn parse_and_handle(
        &mut self,
        packet: RawPacket,
        ctx: &mut Context,
    ) -> Result<(), CommunicationError> {
        // TODO Fix name
        match self.state {
            LoginProtoS2C::PROTOCOL_ID => self.parse_and_handle_login_proto_s2_c(packet, ctx),
            PlayProtoS2C::PROTOCOL_ID => self.parse_and_handle_play_proto_s2_c(packet, ctx),
            _ => Err(ReadError::BadProtocolState.into()),
        }
    }

    fn compression_threshold(&self) -> i32 {
        self.compression_threshold
    }
}

impl<S> PacketHandlerLoginProtoS2C<Context> for Player<S>
where
    for<'a> &'a S: Write,
{
    type Error = CommunicationError;

    fn handle_disconnect_packet(
        &mut self,
        packet: login::DisconnectPacket,
        _: &mut Context,
    ) -> Result<(), Self::Error> {
        Err(CommunicationError::Kicked(packet.reason.to_owned()))
    }

    fn handle_encryption_request_packet(
        &mut self,
        _: login::EncryptionRequestPacket,
        _: &mut Context,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn handle_login_success_packet(
        &mut self,
        packet: login::LoginSuccessPacket,
        _: &mut Context,
    ) -> Result<(), Self::Error> {
        self.uuid = packet.uuid;
        self.username = packet.username.to_owned();
        self.state = PlayProtoS2C::PROTOCOL_ID;

        Ok(())
    }

    fn handle_set_compression_packet(
        &mut self,
        packet: login::SetCompressionPacket,
        _: &mut Context,
    ) -> Result<(), Self::Error> {
        self.compression_threshold = packet.threshold;

        Ok(())
    }

    fn handle_login_plugin_request_packet(
        &mut self,
        _: login::LoginPluginRequestPacket,
        _: &mut Context,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

impl<S> PacketHandlerPlayProtoS2C<Context> for Player<S>
where
    for<'a> &'a S: Write,
{
    type Error = CommunicationError;

    fn handle_disconnect_packet(
        &mut self,
        packet: play::DisconnectPacket,
        _: &mut Context,
    ) -> Result<(), Self::Error> {
        Err(CommunicationError::Kicked(packet.reason.to_owned()))
    }

    fn handle_keep_alive_packet(
        &mut self,
        packet: play::KeepAlivePacket,
        ctx: &mut Context,
    ) -> Result<(), Self::Error> {
        self.ctx_write
            .as_mut()
            .ok_or("Connection writer theft")?
            .write_packet(
                &c2s::play::KeepAlivePacket { id: packet.id },
                &mut ctx.g_write_ctx,
                self.compression_threshold,
            )?;

        Ok(())
    }

    fn handle_join_game_packet(
        &mut self,
        packet: play::JoinGamePacket,
        ctx: &mut Context,
    ) -> Result<(), Self::Error> {
        self.entity_id = packet.entity_id;

        self.ctx_write
            .as_mut()
            .ok_or("Connection writer theft")?
            .write_packet(
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
                &mut ctx.g_write_ctx,
                self.compression_threshold,
            )?;

        Ok(())
    }

    fn handle_teleport_packet(
        &mut self,
        packet: play::TeleportPacket,
        ctx: &mut Context,
    ) -> Result<(), Self::Error> {
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

        self.ctx_write
            .as_mut()
            .ok_or("Connection writer theft")?
            .write_packet(
                &c2s::play::TeleportConfirmPacket { id: packet.id },
                &mut ctx.g_write_ctx,
                self.compression_threshold,
            )?;

        self.should_tick = true;

        Ok(())
    }

    fn handle_time_packet(
        &mut self,
        packet: play::TimePacket,
        _: &mut Context,
    ) -> Result<(), Self::Error> {
        let next = (packet.world_age, Instant::now());
        let last = self.last_game_time;

        let elapsed = next.1 - last.1;
        let tps = (next.0 - last.0) as f64 / elapsed.as_secs_f64();

        self.last_game_time = next;

        if let Some(join_time) = self.join_time {
            if join_time.elapsed() > Duration::from_millis(100) {
                self.tps = tps.min(20.0);
            }
        }

        Ok(())
    }
}

enum Action {
    SendChat,
    Punch,
    ToggleSneak,
    ToggleSprint,
    HeldItem,
}

impl Distribution<Action> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Action {
        let action = rng.gen_range(0..5);
        match action {
            0 => Action::SendChat,
            1 => Action::Punch,
            2 => Action::ToggleSneak,
            3 => Action::ToggleSprint,
            4 => Action::HeldItem,
            _ => unreachable!(),
        }
    }
}
