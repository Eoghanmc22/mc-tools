use crate::player::Player;
use crate::threading::{Worker, ConsoleMessage};
use crate::{threading::BotMessage, Args};
use anyhow::Context;
use log::{warn, info};
use mc_io::error::CommunicationError;
use mc_io::{GlobalReadContext, GlobalWriteContext};
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token, Waker};
use proto::packets::c2s::handshake::HandshakePacket;
use proto::packets::c2s::login::{LoginProtoC2S, LoginStartPacket};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const PROTOCOL_VERSION: u32 = 759;

const WAKER_TOKEN: Token = Token(0);

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);

type Backend = TcpStream;

pub struct BotContext {
    poll: Poll,
}

pub fn setup_bot(worker: &mut Worker) -> anyhow::Result<BotContext> {
    let poll = Poll::new().context("Create poll")?;

    let waker = Waker::new(poll.registry(), WAKER_TOKEN).context("Create waker")?;
    let waker = Arc::new(waker);

    worker.waker = Some(waker);

    Ok(BotContext { poll })
}

pub fn start(ctx: BotContext, args: &Args, worker: Worker) -> anyhow::Result<()> {
    let mut poll = ctx.poll;
    let mut events = Events::with_capacity(500);

    let mut ctx_read = GlobalReadContext::new();
    let mut ctx_write = GlobalWriteContext::new();

    let mut players = HashMap::new();

    loop {
        poll.poll(&mut events, None).context("Poll")?;

        for event in &events {
            match event.token() {
                WAKER_TOKEN => {
                    for message in worker.bot_bound.1.try_iter() {
                        match message {
                            BotMessage::ConnectBot(username) => {
                                let Some((token, player)) =
                                    create_bot(&mut poll, args.server.0, username) 
                                else {
                                        continue;
                                    };
                                players.insert(token, player);
                            }
                            BotMessage::Tick => {
                                for player in players.values_mut() {
                                    let res = player.tick(args, &mut ctx_write);

                                    if let Err(error) = res {
                                        handle_error(player, error, &worker);
                                    }
                                }
                            }
                        }
                    }
                }
                bot_token => {
                    if let Some(player) = players.get_mut(&bot_token) {
                        // Set up the player if needed
                        if event.is_writable() && !player.connected && !player.kicked {
                            let res = connect_bot(player, args.server.0, &mut ctx_write, &worker);

                            if let Err(error) = res {
                                handle_error(player, error, &worker);
                            }
                        }

                        // handle write
                        if event.is_writable() && player.connected && !player.kicked {
                            let res = player.ctx_write.write_unwritten();

                            if let Err(error) = res {
                                handle_error(player, error, &worker);
                            }
                        }

                        // handle read
                        if event.is_readable() && player.connected && !player.kicked {
                            let player_read = player.ctx_read.take();

                            if let Some(mut player_read) = player_read {
                                let res = player_read.read_packets(&mut ctx_read, player);

                                if let Err(error) = res {
                                    handle_error(player, error, &worker);
                                }

                                player.ctx_read = Some(player_read);
                            }
                        }
                    }
                }
            }
        }

        players.retain(|_, player| !player.kicked);
    }
}

fn create_bot(
    poll: &mut Poll,
    server: SocketAddr,
    username: String,
) -> Option<(Token, Player<Backend>)> {
    info!("Starting Bot: {}", username);

    let mut stream = match Backend::connect(server) {
        Ok(stream) => stream,
        Err(error) => {
            warn!("Could not open socket for Bot {}: {}", username, error);
            return None;
        }
    };

    let token = Token(NEXT_TOKEN.fetch_and(1, Ordering::Relaxed));

    poll.registry()
        .register(&mut stream, token, Interest::READABLE | Interest::WRITABLE)
        .expect("Register");

    let player = Player::new(stream, username);

    Some((token, player))
}

fn connect_bot(player: &mut Player<Backend>, server: SocketAddr, ctx: &mut GlobalWriteContext, worker: &Worker) -> Result<(), CommunicationError> {
    match player.socket.peer_addr() {
        Err(err) if err.kind() == ErrorKind::NotConnected => return Ok(()),
        Err(err) => return Err(err.into()),
        _ => (),
    }

    player.socket.set_nodelay(true)?;

    let handshake = HandshakePacket {
        protocol_version: PROTOCOL_VERSION,
        server_address: &server.ip().to_string(),
        server_port: server.port(),
        next_state: LoginProtoC2S::PROTOCOL_ID,
    };

    let login_start = LoginStartPacket {
        username: &player.username,
        signature_data: None,
        uuid: None,
    };

    player.ctx_write.write_packet(&handshake, ctx)?;
    player.ctx_write.write_packet(&login_start, ctx)?;

    info!("Bot Connected: {}", player.username);
    worker.console_bound.0.send(ConsoleMessage::BotConnected).expect("Send msg");

    Ok(())
}

fn handle_error<S>(player: &mut Player<S>, error: CommunicationError, worker: &Worker) {
    player.kicked = true;

    warn!("Bot disconnected {}: {}", player.username, error);
    worker.console_bound.0.send(ConsoleMessage::BotDisconnected).expect("Send msg");
}
