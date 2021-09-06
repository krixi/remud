mod engine;
mod telnet;
mod text;

use ascii::{AsciiString, IntoAsciiString, ToAsciiChar};
use bytes::{Buf, Bytes};
use engine::{db, ClientMessage, Engine, EngineMessage};
use futures::{SinkExt, StreamExt};
use telnet::{Codec, Frame, Telnet};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::codec::Framed;

use crate::engine::ControlMessage;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ClientId(usize);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let pool = db::open("world.db").await?;
    let world = db::load_world(&pool).await?;

    let (engine_tx, engine_rx) = mpsc::channel(256);
    let (control_tx, mut control_rx) = mpsc::channel(16);

    let mut engine = Engine::new(engine_rx, control_tx, world);
    tokio::spawn(async move { engine.run().await });

    let bind_address = "127.0.0.1:2004";
    let listener = TcpListener::bind(bind_address)
        .await
        .unwrap_or_else(|_| panic!("Cannot bind to {:?}", bind_address));
    tracing::info!("Listening on {}", bind_address);

    let mut next_client_id = 1;

    loop {
        tokio::select! {
            Ok((socket, addr)) = listener.accept() => {
                let client_id = ClientId(next_client_id);
                next_client_id += 1;

                let engine_tx = engine_tx.clone();

                tokio::spawn(async move {
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
            }
            message = control_rx.recv() => {
                match message {
                    Some(message) => {
                        match message {
                            ControlMessage::Shutdown =>  {
                                tracing::warn!("Engine shutdown, halting server.");
                                break
                            }
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
                    } else if !telnet.configured() {
                        for frame in telnet.configure() {
                            if framed.send(frame).await.is_err() {
                                break
                            }
                        }
                    } else {
                        ready = true;
                        if tx.send(ClientMessage::Ready(client_id)).await.is_err() {
                            break
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
                buffer.push(char)
            }
        }
    }
}
