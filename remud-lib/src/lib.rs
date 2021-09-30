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

use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use acme_lib::{create_p384_key, persist::FilePersist, Certificate, Directory, DirectoryUrl};
use ascii::{AsciiString, IntoAsciiString, ToAsciiChar};
use bytes::{Buf, Bytes};
use futures::{future::join_all, SinkExt, StreamExt};
use jwt_simple::prelude::ES256KeyPair;
use once_cell::sync::OnceCell;
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
    web::{build_acme_challenge_server, build_web_server},
};

static JWT_KEY_FILE: &str = "jwt_key";

static JWT_KEY: OnceCell<ES256KeyPair> = OnceCell::new();
static TLS_CERT: OnceCell<Certificate> = OnceCell::new();

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct ClientId(usize);

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client {}", self.0)
    }
}

#[derive(Debug, Error)]
pub enum RemudError {
    #[error("engine failed to execute: {0}")]
    EngineError(#[from] EngineError),
    #[error("could not bind to provided address: {0}")]
    BindError(io::Error),
    #[error("could not interact with key: {0}")]
    KeyIoError(io::Error),
    #[error("failed to use JWT key: {0}")]
    JwtKeyError(String),
    #[error("failed to acquire certificate: {0}")]
    CertificateError(#[from] CertificateError),
}

pub async fn run_remud(
    db_path: Option<&str>,
    telnet_port: u16,
    web_port: u16,
    key_path: PathBuf,
    cors: Vec<&str>,
    tls: Option<&str>,
    email: Option<&str>,
    ready_tx: Option<oneshot::Sender<()>>,
) -> Result<(), RemudError> {
    let (engine_tx, engine_rx) = mpsc::channel(256);
    let (control_tx, mut control_rx) = mpsc::channel(16);
    let (web_message_tx, web_message_rx) = mpsc::channel(16);

    load_or_create_jwt_key(key_path.as_path())?;

    let db = Db::new(db_path).await.map_err(EngineError::from)?;

    let mut engine = Engine::new(db.clone(), engine_rx, control_tx, web_message_rx).await?;
    tokio::spawn(async move {
        engine.run().await;
    });
    tracing::debug!("Engine started.");

    let _web_handle = if let Some(domain) = tls {
        if !load_certificate(key_path.as_path(), domain)? {
            let challenge_server = build_acme_challenge_server();
            let challenge_handle =
                tokio::spawn(async move { challenge_server.run(([0, 0, 0, 0], 80)).await });

            if let Err(err) = request_certificate(key_path.as_path(), domain, email.unwrap()) {
                tracing::error!("{}", err);
            }

            challenge_handle.abort();
        }

        // use acme and http validation
        let web_server = build_web_server(db, web_message_tx.clone(), cors)
            .tls()
            .key(TLS_CERT.get().unwrap().private_key())
            .cert(TLS_CERT.get().unwrap().certificate());
        tokio::spawn(async move { web_server.run(([0, 0, 0, 0], web_port)).await })
    } else {
        let web_server = build_web_server(db, web_message_tx.clone(), cors);
        tokio::spawn(async move { web_server.run(([0, 0, 0, 0], web_port)).await })
    };

    let telnet_address = format!("0.0.0.0:{}", telnet_port);
    let telnet_listener = TcpListener::bind(telnet_address.as_str())
        .await
        .map_err(|e| RemudError::BindError(e))?;

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

fn load_or_create_jwt_key(path: &Path) -> Result<(), RemudError> {
    let path = path.join(JWT_KEY_FILE);

    let key = if path.exists() {
        tracing::info!("loading JWT key from: {:?}", path);
        let mut key_file = File::open(path).map_err(|e| RemudError::KeyIoError(e))?;
        let mut key = Vec::new();
        key_file
            .read_to_end(&mut key)
            .map_err(|e| RemudError::KeyIoError(e))?;
        ES256KeyPair::from_bytes(key.as_slice())
            .map_err(|e| RemudError::JwtKeyError(e.to_string()))?
    } else {
        tracing::info!("generating new JWT key, saving to: {:?}", path);
        let key = ES256KeyPair::generate();
        let mut key_file = File::create(path).map_err(|e| RemudError::KeyIoError(e))?;
        key_file
            .write_all(key.to_bytes().as_slice())
            .map_err(|e| RemudError::KeyIoError(e))?;
        key
    };

    if JWT_KEY.set(key).is_err() {
        panic!("unable to set JWT key, key already set");
    };

    Ok(())
}

#[derive(Debug, Error)]
pub enum CertificateError {
    #[error("acme error: {0}")]
    AcmeError(#[from] acme_lib::Error),
    #[error("token save error: {0}")]
    TokenSaveError(#[from] io::Error),
}

fn load_certificate(key_path: &Path, domain: &str) -> Result<bool, CertificateError> {
    let url = DirectoryUrl::LetsEncryptStaging;
    let persist = FilePersist::new(key_path);
    let directory = Directory::from_url(persist, url)?;

    let account = directory.account("sriler@gmail.com")?;
    if let Some(certificate) = account.certificate(domain)? {
        tracing::info!(
            "loading TLS certificate from {} for {}",
            key_path.to_string_lossy(),
            domain
        );
        TLS_CERT.set(certificate).unwrap();
        Ok(true)
    } else {
        tracing::info!(
            "failed to locate TLS certificate in {} for {}",
            key_path.to_string_lossy(),
            domain
        );
        Ok(false)
    }
}

fn request_certificate(key_path: &Path, domain: &str, email: &str) -> Result<(), CertificateError> {
    tracing::info!("requesting new TLS certificate for {}", domain);
    let url = DirectoryUrl::LetsEncrypt;
    let persist = FilePersist::new(key_path);
    let directory = Directory::from_url(persist, url)?;

    let account = directory.account(email)?;

    let mut new_order = account.new_order(domain, &[])?;

    let order_csr = loop {
        if let Some(order_csr) = new_order.confirm_validations() {
            break order_csr;
        }

        let auths = new_order.authorizations()?;
        let challenge = auths[0].http_challenge();

        save_token(challenge.http_token(), challenge.http_proof())?;

        challenge.validate(5000)?;
        new_order.refresh()?;
    };

    let key = create_p384_key();
    let order_certificate = order_csr.finalize_pkey(key, 5000)?;

    let certificate = order_certificate.download_and_save_cert()?;
    TLS_CERT.set(certificate).unwrap();

    tracing::info!(
        "new certificate signed and saved to {}",
        key_path.to_string_lossy()
    );

    Ok(())
}

fn save_token(token: &str, proof: String) -> Result<(), CertificateError> {
    let path = PathBuf::from("acme");
    let mut file = File::create(path.join(token))?;
    writeln!(file, "{}", proof)?;
    Ok(())
}
