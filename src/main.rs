mod engine;
mod telnet;

use bytes::{Buf, Bytes, BytesMut};
use engine::{ClientMessage, Engine, EngineMessage};
use futures::{SinkExt, StreamExt};
use std::collections::VecDeque;
use telnet::{Codec, Frame, Telnet};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (engine_tx, engine_rx) = mpsc::channel(256);

    let mut engine = Engine::new(engine_rx);
    tokio::spawn(async move { engine.run().await });

    let bind_address = "127.0.0.1:2004";
    let listener = TcpListener::bind(bind_address)
        .await
        .unwrap_or_else(|_| panic!("able to bind to {:?}", bind_address));
    tracing::info!("Listening on {}", bind_address);

    let mut client_id = 1;

    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(client_info) => client_info,
            Err(_) => return,
        };

        let engine_tx = engine_tx.clone();

        tokio::spawn(async move {
            tracing::info!("New client ({}): {:?}", client_id, addr);
            let engine_tx = engine_tx;
            let (client_tx, client_rx) = mpsc::unbounded_channel();
            let message = ClientMessage::Connect(client_id, client_tx);
            if engine_tx.send(message).await.is_err() {
                return;
            }

            process(client_id, socket, engine_tx.clone(), client_rx).await;

            let message = ClientMessage::Disconnect(client_id);
            engine_tx.send(message).await.ok();
        });

        client_id += 1;
    }
}

async fn process(
    client_id: usize,
    socket: TcpStream,
    tx: mpsc::Sender<ClientMessage>,
    mut rx: mpsc::UnboundedReceiver<EngineMessage>,
) {
    let mut framed = Framed::new(socket, Codec);
    let mut telnet = Telnet::new();
    let mut ready = false;

    for frame in telnet.initiate() {
        if framed.send(frame).await.is_err() {
            return;
        }
    }

    let mut inputs = VecDeque::new();
    let mut input_buffer = BytesMut::new();

    loop {
        tokio::select! {
            maybe_message = rx.recv() => {
                if let Some(message) = maybe_message {
                    match message {
                        EngineMessage::Output(bytes) => {
                            if framed.send(Frame::Data(bytes)).await.is_err() {
                                break
                            }
                        }
                    }
                } else {
                    let frame = Frame::Data(Bytes::from("Server shutting down. Thanks for playing. <3"));
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
                                while let Some(end) = data
                                    .as_ref()
                                    .windows(2)
                                    .position(|b| b[0] == b'\r' && b[1] == b'\n')
                                {
                                    input_buffer.extend(data.split_to(end));
                                    data.advance(2);

                                    inputs.push_back(input_buffer.freeze());

                                    input_buffer = BytesMut::new();
                                }

                                input_buffer.extend(data);
                            }
                        },
                        Err(e) => {
                            tracing::error!("error decoding frame: {:?}", e);
                            break;
                        }
                    }

                    if ready {
                        // send input and do things
                        for input in inputs.drain(..) {
                            if tx.send(ClientMessage::Input(client_id, input)).await.is_err() {
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
