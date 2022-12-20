use crate::player::Player;
use crate::threading::{Worker, ConsoleMessage};
use crate::{threading::BotMessage, Args};
use anyhow::Context;
use log::{warn, info, error};
use mc_io::error::CommunicationError;
use mc_io::{GlobalReadContext, GlobalWriteContext, PacketHandler};
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token, Waker};
use proto::packets::c2s::handshake::HandshakePacket;
use proto::packets::c2s::login::{LoginProtoC2S, LoginStartPacket};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::mem;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const PROTOCOL_VERSION: u32 = 760;

const WAKER_TOKEN: Token = Token(0);

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);

type Backend<'a> = LoggedStream<'a>;

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
                                    create_bot(&mut poll, args.server.0, username, &worker) 
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
                                player.g_ctx_write = Some(mem::take(&mut ctx_write));

                                let res = player_read.read_packets(&mut ctx_read, player);

                                if let Err(error) = res {
                                    handle_error(player, error, &worker);
                                }

                                player.ctx_read = Some(player_read);

                                if let Some(g_ctx_write) = player.g_ctx_write.take() {
                                    ctx_write = g_ctx_write;
                                } else {
                                    error!("A packet reader stole the globle write context");
                                }
                            }
                        }
                    }
                }
            }
        }

        players.retain(|_, player| !player.kicked);
    }
}

fn create_bot<'a>(
    poll: &mut Poll,
    server: SocketAddr,
    username: String,
    worker: &'a Worker
) -> Option<(Token, Player<Backend<'a>>)> {
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

    let stream = LoggedStream(stream, &worker);
    let player = Player::new(stream, username);

    Some((token, player))
}

fn connect_bot(player: &mut Player<Backend>, server: SocketAddr, ctx: &mut GlobalWriteContext, worker: &Worker) -> Result<(), CommunicationError> {
    match player.socket.0.peer_addr() {
        Err(err) if err.kind() == ErrorKind::NotConnected => return Ok(()),
        Err(err) => return Err(err.into()),
        _ => (),
    }

    player.socket.0.set_nodelay(true)?;


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

    player.ctx_write.write_packet(&handshake, ctx, player.compression_threshold)?;
    player.ctx_write.write_packet(&login_start, ctx, player.compression_threshold)?;

    info!("Bot Connected: {}", player.username);
    worker.console_bound.0.send(ConsoleMessage::BotConnected).expect("Send msg");

    player.connected = true;

    Ok(())
}

fn handle_error<S>(player: &mut Player<S>, error: CommunicationError, worker: &Worker) {
    player.kicked = true;

    warn!("Bot disconnected {}: {}", player.username, error);
    worker.console_bound.0.send(ConsoleMessage::BotDisconnected).expect("Send msg");
}

struct LoggedStream<'a>(pub TcpStream, pub &'a Worker);

impl<'a> Read for LoggedStream<'a> {
    // fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
    //     self.0.read_vectored(bufs)
    // }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.0.read(buf) {
            Ok(amount) => {
                self.1.bytes_rx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            },
            Err(err) => Err(err)
        }
    }
}

impl<'a> Write for LoggedStream<'a> {
    // fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
    //     self.0.write_vectored(bufs)
    // }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.0.write(buf) {
            Ok(amount) => {
                self.1.bytes_tx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            },
            Err(err) => Err(err)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }

}

impl<'a> Read for &LoggedStream<'a> {
    // fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
    //     self.0.read_vectored(bufs)
    // }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // info!("R");
        match (&self.0).read(buf) {
            Ok(amount) => {
                self.1.bytes_rx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            },
            Err(err) => Err(err)
        }
    }
}

impl<'a> Write for &LoggedStream<'a> {
    // fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
    //     self.0.write_vectored(bufs)
    // }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // info!("W");
        match (&self.0).write(buf) {
            Ok(amount) => {
                self.1.bytes_tx.fetch_add(amount as u64, Ordering::Relaxed);
                Ok(amount)
            },
            Err(err) => Err(err)
        }

    }

    fn flush(&mut self) -> std::io::Result<()> {
        (&self.0).flush()
    }

}
