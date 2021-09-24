// #![warn(clippy::pedantic)]
#![allow(clippy::too_many_arguments)]

pub mod color;
mod engine;
mod telnet;
#[cfg(test)]
mod test_e2e;
mod text;
mod web;
mod world;

use std::collections::HashMap;

use ascii::{AsciiString, IntoAsciiString, ToAsciiChar};
use bytes::{Buf, Bytes};
use futures::{future::join_all, SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::codec::Framed;

use crate::{
    color::colorize,
    engine::{ClientMessage, ControlMessage, Engine, EngineMessage},
    telnet::{Codec, Frame, Telnet},
    web::build_web_server,
};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ClientId(usize);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    run().await
}

pub async fn run() -> anyhow::Result<()> {
    let (engine_tx, engine_rx) = mpsc::channel(256);
    let (control_tx, mut control_rx) = mpsc::channel(16);

    let (web_server, web_message_rx) = build_web_server();

    let mut engine = Engine::new(engine_rx, control_tx, web_message_rx).await?;
    tokio::spawn(async move {
        engine.run().await;
    });
    tracing::info!("Engine started.");

    let web_address = "0.0.0.0:2080";
    tokio::spawn(async move {
        match web_server.listen(web_address).await {
            Ok(_) => (),
            Err(e) => tracing::error!("Listen error: {}", e),
        }
    });
    tracing::info!("Web listening on {}", web_address);

    let telnet_address = "0.0.0.0:2004";
    let telnet_listener = TcpListener::bind(telnet_address)
        .await
        .unwrap_or_else(|_| panic!("Cannot bind to {:?}", telnet_address));
    tracing::info!("Telnet listening on {}", telnet_address);

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
                            if framed.send(Frame::Data(Bytes::copy_from_slice(input_buffer.as_bytes()))).await.is_err() {
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
                        Err(e) => {
                            tracing::error!("Error decoding frame: {:?}", e);
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
