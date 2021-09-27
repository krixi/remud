// #![warn(clippy::pedantic)]
#![allow(clippy::too_many_arguments)]

mod color;
mod ecs;
mod engine;
mod macros;
mod telnet;
mod text;
mod web;
mod world;

use std::{collections::HashMap, fmt, io};

use ascii::{AsciiString, IntoAsciiString, ToAsciiChar};
use bytes::{Buf, Bytes};
use futures::{future::join_all, SinkExt, StreamExt};
use jwt_simple::prelude::ES256KeyPair;
use once_cell::sync::Lazy;
use thiserror::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};
use tokio_util::codec::Framed;

use crate::{
    color::colorize,
    engine::{db::Db, ClientMessage, ControlMessage, Engine, EngineError, EngineMessage},
    telnet::{Codec, Frame, Telnet},
    web::build_web_server,
};

static TOKEN_KEY: Lazy<ES256KeyPair> = Lazy::new(|| {
    tracing::info!("Generating Ed25519 key.");
    let key = ES256KeyPair::generate();
    tracing::info!("Key generated.");
    key
});

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct ClientId(usize);

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client {}", self.0)
    }
}

#[derive(Debug, Error)]
pub enum RemudError {
    #[error("could not bind to provided address")]
    BindError(#[from] io::Error),
    #[error("engine failed to execute")]
    EngineError(#[from] EngineError),
}

pub async fn run_remud(
    telnet_port: u16,
    web_port: u16,
    db_path: Option<&str>,
    ready_tx: Option<oneshot::Sender<()>>,
) -> Result<(), RemudError> {
    let (engine_tx, engine_rx) = mpsc::channel(256);
    let (control_tx, mut control_rx) = mpsc::channel(16);

    Lazy::force(&TOKEN_KEY);

    let db = Db::new(db_path).await.map_err(EngineError::from)?;

    let (web_server, web_message_rx) = build_web_server(db.clone());

    let mut engine = Engine::new(db, engine_rx, control_tx, web_message_rx).await?;
    tokio::spawn(async move {
        engine.run().await;
    });
    tracing::debug!("Engine started.");

    tokio::spawn(async move { web_server.run(([0, 0, 0, 0], web_port)).await });
    tracing::debug!("Web listening on 0.0.0.0:{}", web_port);

    let telnet_address = format!("0.0.0.0:{}", telnet_port);
    let telnet_listener = TcpListener::bind(telnet_address.as_str()).await?;
    tracing::debug!("Telnet listening on {}", telnet_address);

    if let Some(tx) = ready_tx {
        tx.send(()).ok();
    }

    let mut next_client_id = 1;
    let mut join_handles = HashMap::new();

    loop {
        tokio::select! {
            Ok((socket, addr)) = telnet_listener.accept() => {
                let client_id = ClientId(next_client_id);
                next_client_id += 1;

                let engine_tx = engine_tx.clone();

                let handle = tokio::spawn(async move {
                    tracing::info!("New client ({:?}): {:?}", client_id, addr);
                    let engine_tx = engine_tx;
                    let (client_tx, client_rx) = mpsc::channel(16);
                    let message = ClientMessage::Connect(client_id, client_tx);
                    if engine_tx.send(message).await.is_err() {
                        return;
                    }

                    process(client_id, socket, engine_tx.clone(), client_rx).await;

                    let message = ClientMessage::Disconnect(client_id);
                    engine_tx.send(message).await.ok();
                });

                join_handles.insert(client_id, handle);
            }
            message = control_rx.recv() => {
                match message {
                    Some(message) => {
                        match message {
                            ControlMessage::Shutdown => {
                                tracing::warn!("Engine shutdown, halting server.");
                                break
                            },
                            ControlMessage::Disconnect(client_id) => {
                                join_handles.remove(&client_id);
                            },
                        }
                    },
                    None => {
                        tracing::error!("Engine failed, halting server");
                        break
                    },
                }
            }
        }
    }

    join_all(join_handles.values_mut()).await;

    Ok(())
}

async fn process(
    client_id: ClientId,
    socket: TcpStream,
    tx: mpsc::Sender<ClientMessage>,
    mut rx: mpsc::Receiver<EngineMessage>,
) {
    let mut framed = Framed::new(socket, Codec);
    let mut telnet = Telnet::new();
    let mut ready = false;

    // Send initial telnet negotiation frames to the client to kick off negotiation
    for frame in telnet.initiate() {
        if framed.send(frame).await.is_err() {
            return;
        }
    }

    let mut inputs = Vec::new();
    let mut input_buffer = AsciiString::new();

    loop {
        tokio::select! {
            maybe_message = rx.recv() => {
                if let Some(message) = maybe_message {
                    match message {
                        EngineMessage::Output(message) => {
                            let message = colorize(format!("|Gray69|{}", message).as_str(), telnet.color_support());
                            match message.into_ascii_string() {
                                Ok(str) => {
                                    let bytes: Vec<u8> = str.into();
                                    if framed.send(Frame::Data(Bytes::from(bytes))).await.is_err() {
                                        break
                                    }
                                },
                                Err(e) => tracing::error!("Engine returned non-ASCII string: \"{}\"", e),
                            }
                        }
                        EngineMessage::EndOutput => {
                            if !input_buffer.is_empty() && framed.send(
                                Frame::Data(Bytes::copy_from_slice(input_buffer.as_bytes()))
                            ).await.is_err() {
                                break
                            }
                        }
                    }
                } else {
                    let frame = Frame::Data(Bytes::from("\r\nServer shutting down. Thanks for playing. <3\r\n"));
                    if framed.send(frame).await.is_err() {
                        break
                    }
                    break
                }
            }
            maybe_frame = framed.next() => {
                if let Some(frame) = maybe_frame {
                    match frame {
                        Ok(frame) => match frame {
                            Frame::Command(_command) => (),
                            Frame::Negotiate(command, option) => {
                                for frame in telnet.negotiate(command, option) {
                                    if framed.send(frame).await.is_err() {
                                        break
                                    }
                                }
                            }
                            Frame::Subnegotiate(option, data) => {
                                for frame in telnet.subnegotiate(option, data) {
                                    if framed.send(frame).await.is_err() {
                                        break
                                    }
                                }
                            }
                            Frame::Data(mut data) => {
                                while let Some(end_of_command) = data
                                    .as_ref()
                                    .windows(2)
                                    .position(|b| b[0] == b'\r' && b[1] == b'\n')
                                {
                                    let rest_of_command = data.split_to(end_of_command);
                                    append_input(rest_of_command, &mut input_buffer);
                                    inputs.push(input_buffer);

                                    data.advance(2);

                                    input_buffer = AsciiString::new();
                                }

                                append_input(data, &mut input_buffer);
                            }
                        },
                        Err(_) => {
                            tracing::info!("Client disconnected");
                            break;
                        }
                    }

                    if ready {
                        // send input and do things
                        for input in inputs.drain(..) {
                            if tx.send(ClientMessage::Input(client_id, input.to_string())).await.is_err() {
                                break
                            }
                        }
                    } else if telnet.configured() {
                        ready = true;
                        if tx.send(ClientMessage::Ready(client_id)).await.is_err() {
                            break
                        }
                    } else {
                        for frame in telnet.configure() {
                            if framed.send(frame).await.is_err() {
                                break
                            }
                        }
                    }
                } else {
                    // TcpStream closed
                    break
                }
            }
        }
    }
}

fn append_input(input: Bytes, buffer: &mut AsciiString) {
    for byte in input {
        if let Ok(char) = byte.to_ascii_char() {
            if char.is_ascii_control() {
                if matches!(char, ascii::AsciiChar::BackSpace) {
                    buffer.pop();
                }
            } else {
                buffer.push(char);
            }
        }
    }
}
