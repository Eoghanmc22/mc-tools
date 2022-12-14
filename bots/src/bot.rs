use crate::context::Context as WorkerContext;
use crate::player::Player;
use crate::threading::{ConsoleMessage, Worker};
use crate::{threading::BotMessage, Args};
use anyhow::Context;
use log::{info, warn};
use mc_io::error::CommunicationError;
use mc_io::{GlobalReadContext, GlobalWriteContext};
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token, Waker};
use proto::packets::c2s::handshake::HandshakePacket;
use proto::packets::c2s::login::{LoginProtoC2S, LoginStartPacket};

use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::SocketAddr;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

const PROTOCOL_VERSION: u32 = 760;

const WAKER_TOKEN: Token = Token(0);

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);

type Backend = LoggedStream;

pub struct BotContext {
    poll: Poll,
}

pub fn setup_bot(worker: &mut Worker) -> anyhow::Result<BotContext> {
    let poll = Poll::new().context("Create poll")?;

    let waker = Waker::new(poll.registry(), WAKER_TOKEN).context("Create waker")?;
    worker.waker = Some(waker);

    Ok(BotContext { poll })
}

pub fn start(ctx: BotContext, args: &Args, worker: Arc<Worker>) -> anyhow::Result<()> {
    let mut poll = ctx.poll;
    let mut events = Events::with_capacity(500);

    let mut ctx_read = GlobalReadContext::new();

    let mut players = HashMap::new();

    let mut last_tick = Instant::now();

    let ctx_write = GlobalWriteContext::new();
    let messages = generate_messages(args).context("Could not generate chat messages")?;
    let mut context = WorkerContext {
        messages,
        g_write_ctx: ctx_write,
    };

    'main_loop: loop {
        poll.poll(&mut events, None).context("Poll")?;

        for event in &events {
            match event.token() {
                WAKER_TOKEN => {
                    for message in worker.bot_bound.1.try_iter() {
                        match message {
                            BotMessage::ConnectBot(username) => {
                                if let Some((token, player)) =
                                    create_bot(&mut poll, args.server.0, username, &worker)
                                {
                                    players.insert(token, player);
                                }
                            }
                            BotMessage::Tick => {
                                let mut tps_total = 0.0;
                                let mut tps_count = 0;

                                for player in players.values_mut() {
                                    let res = player.tick(args, &mut context);

                                    if player.last_game_time.1 > last_tick && !player.tps.is_nan() {
                                        tps_total += player.tps;
                                        tps_count += 1;
                                    }

                                    if let Err(error) = res {
                                        handle_error(player, error, &worker);
                                    }
                                }

                                if tps_count != 0 {
                                    worker
                                        .console_bound
                                        .0
                                        .send(ConsoleMessage::TPS(tps_total, tps_count))
                                        .context("Send msg")?;
                                }

                                last_tick = Instant::now();
                            }
                            BotMessage::Stop => {
                                break 'main_loop;
                            }
                        }
                    }
                }
                bot_token => {
                    if let Some(player) = players.get_mut(&bot_token) {
                        // Set up the player if needed
                        if event.is_writable() && !player.connected && !player.kicked {
                            let res = connect_bot(
                                player,
                                args.server.0,
                                &mut context,
                                &worker,
                                args.proto_id.unwrap_or(PROTOCOL_VERSION),
                            );

                            if let Err(error) = res {
                                handle_error(player, error, &worker);
                            }
                        }

                        // handle write
                        if event.is_writable() && !player.kicked {
                            let res = player
                                .ctx_write
                                .as_mut()
                                .context("Connection writer theft")?
                                .write_unwritten();

                            if let Err(error) = res {
                                handle_error(player, error, &worker);
                            }
                        }

                        // handle read
                        if event.is_readable() && !player.kicked {
                            let player_read = player.ctx_read.take();

                            if let Some(mut player_read) = player_read {
                                let res =
                                    player_read.read_packets(&mut ctx_read, player, &mut context);

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

    Ok(())
}

fn generate_messages(args: &Args) -> anyhow::Result<Vec<String>> {
    if let Some(name) = &args.message_file {
        let file = fs::read_to_string(name).context("Could not read `message_file`")?;
        Ok(file.lines().map(|it| it.to_owned()).collect())
    } else {
        Ok(vec![
            "This is a chat message!".to_owned(),
            "Wow".to_owned(),
            "Idk what to put here".to_owned(),
        ])
    }
}

fn create_bot(
    poll: &mut Poll,
    server: SocketAddr,
    username: String,
    worker: &Arc<Worker>,
) -> Option<(Token, Player<Backend>)> {
    info!("Starting Bot: {}", username);

    let mut stream = match TcpStream::connect(server) {
        Ok(stream) => stream,
        Err(error) => {
            warn!("Could not open socket for Bot {}: {}", username, error);
            return None;
        }
    };

    let token = Token(NEXT_TOKEN.fetch_add(1, Ordering::Relaxed));

    poll.registry()
        .register(&mut stream, token, Interest::READABLE | Interest::WRITABLE)
        .expect("Register");

    let stream = LoggedStream(stream, worker.clone());
    let player = Player::new(stream, username);

    Some((token, player))
}

fn connect_bot(
    player: &mut Player<Backend>,
    server: SocketAddr,
    ctx: &mut WorkerContext,
    worker: &Worker,
    protocol_version: u32,
) -> Result<(), CommunicationError> {
    match player.socket.0.peer_addr() {
        Err(err) if err.kind() == ErrorKind::NotConnected => return Ok(()),
        Err(err) => return Err(err.into()),
        _ => (),
    }

    player.socket.0.set_nodelay(true)?;

    let handshake = HandshakePacket {
        protocol_version,
        server_address: &server.ip().to_string(),
        server_port: server.port(),
        next_state: LoginProtoC2S::PROTOCOL_ID,
    };

    let login_start = LoginStartPacket {
        username: &player.username,
        signature_data: None,
        uuid: None,
    };

    player
        .ctx_write
        .as_mut()
        .ok_or("Connection writer theft")?
        .write_packets(
            &mut ctx.g_write_ctx,
            player.compression_threshold,
            |writer| {
                writer.write_packet(&handshake)?;
                writer.write_packet(&login_start)?;

                Ok(())
            },
        )?;

    info!("Bot Connected: {}", player.username);
    worker
        .console_bound
        .0
        .send(ConsoleMessage::BotConnected)
        .expect("Send msg");

    player.connected = true;
    player.join_time = Some(Instant::now());

    Ok(())
}

fn handle_error<S>(player: &mut Player<S>, error: CommunicationError, worker: &Worker) {
    player.kicked = true;

    warn!("Bot encountered error {}: {}", player.username, error);

    if player.connected {
        worker
            .console_bound
            .0
            .send(ConsoleMessage::BotDisconnected)
            .expect("Send msg");
    }
}

struct LoggedStream(pub TcpStream, pub Arc<Worker>);

impl Read for LoggedStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.0.read(buf) {
            Ok(amount) => {
                self.1.bytes_rx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            }
            Err(err) => Err(err),
        }
    }
}

impl Write for LoggedStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.0.write(buf) {
            Ok(amount) => {
                self.1.bytes_tx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            }
            Err(err) => Err(err),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl Read for &LoggedStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match (&self.0).read(buf) {
            Ok(amount) => {
                self.1.bytes_rx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            }
            Err(err) => Err(err),
        }
    }
}

impl Write for &LoggedStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match (&self.0).write(buf) {
            Ok(amount) => {
                self.1.bytes_tx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            }
            Err(err) => Err(err),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        (&self.0).flush()
    }
}
