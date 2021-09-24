use std::{
    io::{self},
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

use once_cell::sync::Lazy;
use remud_lib::{run_remud, RemudError};
use telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};
use tokio::sync::oneshot;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter, FmtSubscriber};

static PORT_COUNTER: Lazy<AtomicU16> = Lazy::new(|| AtomicU16::new(49152));
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "remud_lib=trace".to_string();

    init_subscriber(default_filter_level.into(), TestWriter::default());
});

/// spawn the server and wait for it to start
pub async fn start_server() -> (u16, u16) {
    Lazy::force(&TRACING);

    #[allow(unused_assignments)]
    let mut telnet_port = 0;
    #[allow(unused_assignments)]
    let mut web_port = 0;

    'connect_loop: loop {
        let (tx, rx) = oneshot::channel();
        telnet_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let telnet_addr = format!("127.0.0.1:{}", telnet_port);
        web_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let web_addr = format!("127.0.0.1:{}", web_port);
        let spawn = tokio::spawn(async move {
            let telnet_addr = telnet_addr;
            run_remud(&telnet_addr, &web_addr, None, Some(tx)).await
        });

        tokio::select! {
            join_result = spawn => {
                match join_result {
                    Ok(remud_result) => {
                        match remud_result {
                            // ReMUD did not stop to listen for requests and the run function returned early.
                            Ok(_) => panic!("ReMUD exited early"),
                            Err(e) => match e {
                                RemudError::BindError(_) => {
                                    tracing::info!("port {} or {} in use, selecting next ports", telnet_port, web_port);
                                },
                                RemudError::EngineError(e) => panic!("ReMUD failed to start: {}", e)
                            }
                        }
                    }
                    Err(_) => {
                        panic!("Failed to join ReMUD task")
                    }
                }
            }
            _ = rx => { break 'connect_loop }
        }
    }

    (telnet_port, web_port)
}

pub struct TelnetClient {
    connection: Telnet,
}

impl TelnetClient {
    // Creates a new client and performs initial TELNET options negotiation.
    pub fn new(port: u16) -> Self {
        // set up connection
        let mut connection =
            Telnet::connect(("127.0.0.1", port), 1024).expect("failed to connect to ReMUD");

        if let TelnetEvent::Negotiation(NegotiationAction::Do, TelnetOption::TTYPE) = connection
            .read_timeout(Duration::from_secs(1))
            .expect("did not receive DO TTYPE")
        {
            connection.negotiate(NegotiationAction::Wont, TelnetOption::TTYPE);
        } else {
            panic!("received unexpected message waiting for DO TTYPE");
        }

        TelnetClient { connection }
    }

    pub fn recv_contains(&mut self, text: &str) {
        let event = self
            .connection
            .read_timeout(Duration::from_secs(1))
            .unwrap_or_else(|_| {
                panic!(
                    "failed to read from telnet connection while looking for '{}'",
                    text,
                )
            });

        if let TelnetEvent::Data(data) = event {
            let message =
                String::from_utf8(data.to_vec()).expect("server sent invalid UTF-8 string");
            assert!(
                message.contains(text),
                "did not find '{}' in message {:?}",
                text,
                message,
            )
        } else {
            panic!(
                "did not receive expected DATA event containing '{}': {:?}",
                text, event
            );
        }
    }

    pub fn send(&mut self, line: &str) {
        self.connection
            .write(format!("{}\r\n", line).as_bytes())
            .unwrap_or_else(|_| panic!("failed to send '{}'", line));
    }

    pub fn recv_prompt(&mut self) {
        self.recv_contains(">");
    }

    pub fn create_user(&mut self, name: &str, password: &str) {
        self.recv_contains("Name?");
        self.send(name);
        self.recv_contains("Password?");
        self.send(password);
        self.recv_contains("Verify?");
        self.send(password);
    }
}

#[derive(Default)]
pub struct TestWriter {}

impl io::Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!("{}", String::from_utf8(buf.to_vec()).unwrap());

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeWriter for TestWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        TestWriter::default()
    }
}

pub fn init_subscriber(env_filter: String, sink: impl MakeWriter + Send + Sync + 'static) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(sink)
        .with_level(true)
        .init()
}