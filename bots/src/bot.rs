use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;
use anyhow::Context;
use crossbeam::channel::{Receiver, Sender};
use log::warn;
use crate::{Args, address::MinecraftAddress, channels::BotMessage};
use mio::{Token, Waker};
use mio::net::TcpStream;
use mio_misc::{channel, NotificationId};
use mio_misc::channel::CrossbeamSender;
use mio_misc::poll::Poll;
use mio_misc::queue::BoundedNotificationQueue;
use mio_misc::scheduler::{NotificationScheduler, Scheduler};
use mc_io::error::CommunicationError;
use mc_io::{GlobalContext, packet};
use crate::channels::ConsoleMessage;
use crate::player::Player;
use protocol::*;
use mc_io::io::{read, write};

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

pub fn setup_bot(args: Args, to_console: Sender<ConsoleMessage>, scheduler: Arc<Scheduler>) -> anyhow::Result<(Bot, BotContext)> {
    let Args { server: MinecraftAddress(server), radius, speed, .. } = args;

    let poll = Poll::with_capacity(100).context("Create poll")?;

    let waker = Waker::new(poll.registry(), WAKER_TOKEN).context("Create waker")?;
    let waker = Arc::new(waker);

    let queue = BoundedNotificationQueue::new(100, waker.clone());
    let queue = Arc::new(queue);

    let from_console_id = NotificationId::gen_next();
    let (to_bot, from_console) = channel::crossbeam_channel_bounded(queue.clone(), from_console_id, 100);

    let tick_id = NotificationId::gen_next();
    NotificationScheduler::new(queue.clone(), scheduler.clone())
        .notify_with_fixed_interval(
            tick_id,
            TICK_DURATION,
            None,
            Some("Tick Scheduler".to_owned())
        );

    let bot = Bot {
        to_bot,
    };

    let ctx = BotContext {
        server,
        radius,
        speed,
        poll,
        queue,
        to_console,
        from_console,
        from_console_id,
        tick_id
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
        tick_id
    } = ctx;

    let mut global_ctx = GlobalContext::new();
    let mut players: HashMap<Token, Player<TcpStream>> = HashMap::new();

    let events = poll.poll(None).expect("Poll events");
    for event in events {
        match event.token() {
            WAKER_TOKEN => {
                todo!()
            }
            bot_token => {
                if let Some(player) = players.get_mut(&bot_token) {
                    // Set up the player if needed
                    if event.is_writable() && !player.connected && !player.kicked {
                        let result = || -> Result<(), CommunicationError> {
                            player.connection.socket.set_nodelay(true)?;

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

                            let (buffer, mut compression_ctx) = global_ctx.compression(&player.connection);
                            packet::write_packet(&handshake, buffer, &mut compression_ctx)?;
                            packet::write_packet(&login_start, buffer, &mut compression_ctx)?;

                            player.connection.write_buffer(buffer)?;

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
                            write::write(&mut player.connection)?;

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
                            read::read(&mut global_ctx, &mut player.connection, |packet, write_buf, compression_ctx| {
                                Ok(())
                            })
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
}
