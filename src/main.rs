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

    let (engine_tx, engine_rx) = mpsc::unbounded_channel();

    let mut engine = Engine::new(engine_rx);
    tokio::spawn(async move { engine.run().await });

    let bind_address = "127.0.0.1:2004";
    let listener = TcpListener::bind(bind_address)
        .await
        .expect(format!("able to bind to {:?}", bind_address).as_str());
    tracing::info!("Listening on {}", bind_address);

    let mut client_id = 1;

    loop {
        let (socket, _) = match listener.accept().await {
            Ok(client_info) => client_info,
            Err(_) => return,
        };

        let engine_tx = engine_tx.clone();

        tokio::spawn(async move {
            let engine_tx = engine_tx;
            let (client_tx, client_rx) = mpsc::unbounded_channel();
            match engine_tx.send(ClientMessage::Connect(client_id, client_tx)) {
                Err(_) => return,
                _ => (),
            }

            process(client_id, socket, engine_tx.clone(), client_rx).await;

            match engine_tx.send(ClientMessage::Disconnect(client_id)) {
                Err(_) => return,
                _ => (),
            }
        });

        client_id += 1;
    }
}

async fn process(
    client_id: usize,
    socket: TcpStream,
    tx: mpsc::UnboundedSender<ClientMessage>,
    mut rx: mpsc::UnboundedReceiver<EngineMessage>,
) {
    let mut framed = Framed::new(socket, Codec);
    let mut telnet = Telnet::new();
    let mut ready = false;

    for frame in telnet.initiate() {
        match framed.send(frame).await {
            Err(_) => return,
            _ => (),
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
                            match framed.send(Frame::Data(bytes)).await {
                                Err(_) => break,
                                _ => ()
                            }
                        }
                    }
                } else {
                    match framed.send(Frame::Data(Bytes::from("Server shutting down. Thanks for playing. <3"))).await {
                        Err(_) => break,
                        _ => ()
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
                                    match framed.send(frame).await {
                                        Err(_) => break,
                                        _ => ()
                                    }
                                }
                            }
                            Frame::Subnegotiate(option, data) => {
                                for frame in telnet.subnegotiate(option, data) {
                                    match framed.send(frame).await {
                                        Err(_) => break,
                                        _ => ()
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
                            tracing::error!("err: {:?}", e);
                            break;
                        }
                    }

                    if ready {
                        // send input and do things
                        for input in inputs.drain(..) {
                            match tx.send(ClientMessage::Input(client_id, input)) {
                                Err(_) => break,
                                _ => ()
                            }
                        }
                    } else if !telnet.configured() {
                        for frame in telnet.configure() {
                            match framed.send(frame).await {
                                Err(_) => break,
                                _ => ()
                            }
                        }
                    } else {
                        ready = true;
                        match tx.send(ClientMessage::Ready(client_id)) {
                            Err(_) => break,
                            _ => ()
                        };
                    }
                } else {
                    break
                }
            }
        }
    }
}
