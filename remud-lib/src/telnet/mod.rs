mod protocol;

use std::{io, sync::atomic::Ordering};

use ascii::{AsciiString, IntoAsciiString, ToAsciiChar};
use bytes::{Buf, Bytes};
use futures::{FutureExt, SinkExt, StreamExt};
use thiserror::Error;
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::mpsc,
    task::JoinHandle,
};
use tokio_util::codec::Framed;

use crate::{
    color::colorize_telnet,
    engine::{ClientMessage, EngineResponse, Output},
    telnet::protocol::{Codec, Frame, Telnet},
    ClientId, CLIENT_ID_COUNTER,
};

pub struct Server {
    listener: TcpListener,
}

impl Server {
    #[tracing::instrument(name = "initializing telnet server", skip_all)]
    pub async fn new<A: ToSocketAddrs>(address: A) -> Result<Self, Error> {
        let listener = TcpListener::bind(address).await?;

        Ok(Server { listener })
    }

    #[tracing::instrument(name = "accepting telnet connection", skip_all)]
    pub(crate) async fn accept(
        &self,
        client_tx: mpsc::Sender<ClientMessage>,
    ) -> Option<(ClientId, JoinHandle<()>)> {
        self.listener
            .accept()
            .map(|connection| match connection {
                Ok((stream, address)) => {
                    let client_id = ClientId(CLIENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst));

                    let handle = tokio::spawn(async move {
                        tracing::info!("new client ({:?}): {:?}", client_id, address);
                        let client_tx = client_tx;
                        let (engine_tx, engine_rx) = mpsc::channel(16);

                        let message =
                            ClientMessage::Connect(client_id, client_tx.clone(), engine_tx);
                        if client_tx.send(message).await.is_err() {
                            return;
                        }

                        stream.set_nodelay(true).unwrap();

                        process(client_id, stream, client_tx.clone(), engine_rx).await;

                        let message = ClientMessage::Disconnect(client_id);
                        client_tx.send(message).await.ok();
                    });
                    Some((client_id, handle))
                }
                Err(e) => {
                    tracing::error!("failed to get new client: {}", e);
                    None
                }
            })
            .await
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to bind Telnet socket")]
    BindError(#[from] io::Error),
}

#[tracing::instrument(
    name = "processing telnet connection",
    skip(socket, client_tx, engine_rx)
)]
async fn process(
    client_id: ClientId,
    socket: TcpStream,
    client_tx: mpsc::Sender<ClientMessage>,
    mut engine_rx: mpsc::Receiver<EngineResponse>,
) {
    let mut framed = Framed::new(socket, Codec);
    let mut telnet = Telnet::new();
    let mut ready = false;
    let mut awaiting_input = false;

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
            maybe_message = engine_rx.recv() => {
                if let Some(message) = maybe_message {
                    match message {
                        EngineResponse::Output(outputs) => {
                            // If awaiting input, add a newline to move past prompt
                            if awaiting_input {
                                awaiting_input = false;
                                if framed.send(Frame::Data(Bytes::from("\r\n"))).await.is_err() {
                                    break;
                                }
                            }

                            // Send all output to the client
                            for output in outputs {
                                let message = match output {
                                    Output::Message(message) => colorize_telnet(format!("|Gray69|{}|-|\r\n", message.as_str()).as_str(), telnet.color_support()),
                                    // what to do to make telnet hide this input when sensitive?
                                    Output::Prompt{format, ..} => {
                                        awaiting_input = true;
                                        colorize_telnet(format!("|Gray69|{}|-|", format.as_str()).as_str(), telnet.color_support())
                                    }
                                };

                                match message.into_ascii_string() {
                                    Ok(str) => {
                                        let bytes: Vec<u8> = str.into();
                                        if framed.send(Frame::Data(Bytes::from(bytes))).await.is_err() {
                                            break;
                                        }
                                    },
                                    Err(e) => tracing::error!("Engine returned non-ASCII string: \"{}\"", e),
                                }
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
                            tracing::info!("client disconnected");
                            break;
                        }
                    }

                    if ready {
                        // send input and do things
                        for input in inputs.drain(..) {
                            awaiting_input = false;
                            if client_tx.send(ClientMessage::Input(client_id, input.to_string())).await.is_err() {
                                break
                            }
                        }
                    } else if telnet.configured() {
                        ready = true;
                        if client_tx.send(ClientMessage::Ready(client_id)).await.is_err() {
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
