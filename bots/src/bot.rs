use crate::channels::ConsoleMessage;
use crate::player::Player;
use crate::{address::MinecraftAddress, channels::BotMessage, Args};
use anyhow::Context;
use binary::slice_serialization::SliceSerializable;
use crossbeam::channel::{Receiver, Sender};
use euclid::Vector3D;
use log::{error, warn};
use mc_io::buf::Buffer;
use mc_io::error::{CommunicationError, ReadError};
use mc_io::io::{read, write};
use mc_io::{
    packet, CompressionContext, ConnectionContext, FramedPacket, GlobalContext, RawPacket,
};
use mio::net::TcpStream;
use mio::{Interest, Token, Waker};
use mio_misc::channel::CrossbeamSender;
use mio_misc::poll::Poll;
use mio_misc::queue::{BoundedNotificationQueue, NotificationReceiver};
use mio_misc::scheduler::{NotificationScheduler, Scheduler};
use mio_misc::{channel, NotificationId};
use protocol::types::{ArmPosition, ChatVisibility};
use protocol::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const PROTOCOL_VERSION: i32 = 759;

const WAKER_TOKEN: Token = Token(0);
const TICK_DURATION: Duration = Duration::from_millis(50);

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);

pub struct Bot {
    pub to_bot: CrossbeamSender<BotMessage>,
}

pub struct BotContext {
    server: SocketAddr,
    radius: usize,
    speed: f64,

    poll: Poll,
    queue: Arc<BoundedNotificationQueue>,

    to_console: Sender<ConsoleMessage>,
    from_console: Receiver<BotMessage>,

    from_console_id: NotificationId,
    tick_id: NotificationId,
}

pub fn setup_bot(
    args: Args,
    to_console: Sender<ConsoleMessage>,
    scheduler: Arc<Scheduler>,
) -> anyhow::Result<(Bot, BotContext)> {
    let Args {
        server: MinecraftAddress(server),
        radius,
        speed,
        ..
    } = args;

    let poll = Poll::with_capacity(100).context("Create poll")?;

    let waker = Waker::new(poll.registry(), WAKER_TOKEN).context("Create waker")?;
    let waker = Arc::new(waker);

    let queue = BoundedNotificationQueue::new(100, waker.clone());
    let queue = Arc::new(queue);

    let from_console_id = NotificationId::gen_next();
    let (to_bot, from_console) =
        channel::crossbeam_channel_bounded(queue.clone(), from_console_id, 100);

    let tick_id = NotificationId::gen_next();
    NotificationScheduler::new(queue.clone(), scheduler.clone()).notify_with_fixed_interval(
        tick_id,
        TICK_DURATION,
        None,
        Some("Tick Scheduler".to_owned()),
    );

    let bot = Bot { to_bot };

    let ctx = BotContext {
        server,
        radius,
        speed,
        poll,
        queue,
        to_console,
        from_console,
        from_console_id,
        tick_id,
    };

    Ok((bot, ctx))
}

pub fn start(ctx: BotContext) {
    let BotContext {
        server,

        radius,
        speed,

        mut poll,
        queue,

        to_console,
        from_console,

        from_console_id,
        tick_id,
    } = ctx;

    let mut global_ctx = GlobalContext::new();
    let mut players: HashMap<Token, (Player, ConnectionContext<TcpStream>)> = HashMap::new();
    let mut unregistered_streams = Vec::new();

    loop {
        let events = poll.poll(None).expect("Poll events");
        for event in events {
            match event.token() {
                WAKER_TOKEN => {
                    // They didnt implement iter() grrr
                    while let Some(notification) = queue.receive() {
                        if notification == from_console_id {
                            for message in &from_console {
                                match message {
                                    BotMessage::ConnectBot(username) => {
                                        let mut stream = match TcpStream::connect(server) {
                                            Ok(stream) => stream,
                                            Err(error) => {
                                                error!(
                                                    "Bot `{username}` could not connect: {error}"
                                                );
                                                continue;
                                            }
                                        };
                                        let token =
                                            Token(NEXT_TOKEN.fetch_and(1, Ordering::Relaxed));
                                        //poll.registry()
                                        //.register(&mut stream, token, Interest::READABLE | Interest::WRITABLE).expect("Register");
                                        let player = Player {
                                            entity_id: -1,
                                            proto_state: 2,
                                            username,
                                            connected: false,
                                            teleported: false,
                                            kicked: false,
                                            position: Default::default(),
                                            velocity: Vector3D {
                                                x: rand::random(),
                                                y: 0.0,
                                                z: rand::random(),
                                                ..Default::default()
                                            } * speed,
                                        };
                                        let connection = ConnectionContext::new(stream);

                                        unregistered_streams.push(token);
                                        players.insert(token, (player, connection));
                                    }
                                }
                            }
                        } else if notification == tick_id {
                            for (player, connection) in players.values_mut() {
                                if !player.teleported || player.kicked {
                                    continue;
                                }

                                player.position += player.velocity;

                                if player.position.x.abs() > radius as f64 {
                                    player.velocity.x = -player.velocity.x;
                                }
                                if player.position.z.abs() > radius as f64 {
                                    player.velocity.z = -player.velocity.z;
                                }

                                let (write_buffer, mut compression_ctx) =
                                    global_ctx.compression(connection);
                                // TODO: Send a variety of move packets
                                let move_packet = play::client::MovePlayerPos {
                                    x: player.position.x,
                                    y: player.position.y,
                                    z: player.position.z,
                                    on_ground: false,
                                };

                                // TODO: Implement random actions
                            }
                        } else {
                            error!("Networking thread got unknown notification")
                        }
                    }
                }
                bot_token => {
                    if let Some((player, connection)) = players.get_mut(&bot_token) {
                        // Set up the player if needed
                        if event.is_writable() && !player.connected && !player.kicked {
                            let result = || -> Result<(), CommunicationError> {
                                // FIXME check TcpStream::peer_addr() for errors before actually
                                // labaling the connection as connected
                                connection.socket.set_nodelay(true)?;

                                let handshake = handshake::client::Intention {
                                    protocol_version: PROTOCOL_VERSION,
                                    host_name: "",
                                    port: 25565,
                                    intention: 2,
                                };

                                let login_start = login::client::Hello {
                                    username: &player.username,
                                    signature_data: None,
                                    uuid: None,
                                };

                                let (buffer, mut compression_ctx) =
                                    global_ctx.compression(&connection);
                                packet::write_packet(&handshake, buffer, &mut compression_ctx)?;
                                packet::write_packet(&login_start, buffer, &mut compression_ctx)?;

                                connection.write_buffer(buffer)?;

                                Ok(())
                            }();

                            if let Err(error) = result {
                                warn!("Bot {} was kicked: {}", player.username, error);
                                player.kicked = true;
                            }
                        }

                        // handle write
                        if event.is_writable() && player.connected && !player.kicked {
                            let result = || -> Result<(), CommunicationError> {
                                write::write(connection)?;

                                Ok(())
                            }();

                            if let Err(error) = result {
                                warn!("Bot {} was kicked: {}", player.username, error);
                                player.kicked = true;
                            }
                        }

                        // handle read
                        if event.is_readable() && player.connected && !player.kicked {
                            let result = || -> Result<(), CommunicationError> {
                                read::read(
                                    &mut global_ctx,
                                    connection,
                                    |packet, write_buf, compression| {
                                        handle_packet(packet, player, write_buf, compression)
                                    },
                                )?;

                                Ok(())
                            }();

                            if let Err(error) = result {
                                warn!("Bot {} was kicked: {}", player.username, error);
                                player.kicked = true;
                            }
                        }
                    }
                }
            }
        }

        todo!("Purge kicked players");
        todo!("Register streams");
    }
}

// TODO: Improve
fn handle_packet(
    packet: &FramedPacket,
    player: &mut Player,
    write_buf: &mut Buffer,
    compression: &mut CompressionContext,
) -> Result<(), CommunicationError> {
    let RawPacket(id, mut packet) = packet::read_packet(packet, compression)?;

    match player.proto_state {
        // Handshake
        0 => return Err(ReadError::BadProtocolState.into()),
        // Status
        1 => return Err(ReadError::BadProtocolState.into()),
        // Login
        2 => {
            match login::server::PacketId::try_from(id).map_err(|_| ReadError::BadPacketID(id))? {
                login::server::LoginSuccess::ID => {
                    player.proto_state = 3;
                }
                /*login::server::Disconnect::ID => {

                }*/
                /*login::server::SetCompression::ID => {

                },*/
                _ => {}
            }
        }
        // Play
        3 => {
            match play::server::PacketId::try_from(id).map_err(|_| ReadError::BadPacketID(id))? {
                play::server::KeepAlive::ID => {
                    let packet =
                        play::server::KeepAlive::read(&mut packet).map_err(ReadError::BadPacket)?;
                    let response = play::client::KeepAlive { id: packet.id };
                    packet::write_packet(&response, write_buf, compression)?;
                }
                play::server::Login::ID => {
                    let packet =
                        play::server::Login::read(&mut packet).map_err(ReadError::BadPacket)?;

                    player.entity_id = packet.entity_id;

                    let response = play::client::ClientInformation {
                        language: "en_US",
                        view_distance: 10,
                        chat_visibility: ChatVisibility::default(),
                        chat_colors: true,
                        model_customization: 0b00111111,
                        arm_position: ArmPosition::default(),
                        text_filtering_enabled: false,
                        show_on_server_list: true,
                    };
                    packet::write_packet(&response, write_buf, compression)?;
                }
                /*play::server::Disconnect::ID => {

                }*/
                play::server::PlayerPosition::ID => {
                    let packet = play::server::PlayerPosition::read(&mut packet)
                        .map_err(ReadError::BadPacket)?;

                    if packet.relative_arguments & 0b10000 == 0 {
                        player.position.x = packet.x;
                    } else {
                        player.position.x += packet.x;
                    }
                    if packet.relative_arguments & 0b01000 == 0 {
                        player.position.y = packet.y;
                    } else {
                        player.position.y += packet.y;
                    }
                    if packet.relative_arguments & 0b00100 == 0 {
                        player.position.z = packet.z;
                    } else {
                        player.position.z += packet.z;
                    }

                    let response = play::client::AcceptTeleportation { id: packet.id };
                    packet::write_packet(&response, write_buf, compression)?;
                }
                _ => {}
            }
        }
        _ => return Err(ReadError::BadProtocolState.into()),
    }

    Ok(())
}

fn handle_tick(player: &mut Player, write_buf: &mut Buffer, compression: &mut CompressionContext) {}
