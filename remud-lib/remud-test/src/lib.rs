use std::{
    io::{self},
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

use itertools::Itertools;
use once_cell::sync::Lazy;
use remud_lib::{run_remud, RemudError};
use telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};
use tokio::sync::oneshot;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter, FmtSubscriber};

static PORT_COUNTER: Lazy<AtomicU16> = Lazy::new(|| AtomicU16::new(49152));
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "remud_lib=trace".to_string();

    init_subscriber(default_filter_level, TestWriter::default());
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

    pub fn recv(&mut self) -> String {
        let event = self
            .connection
            .read_timeout(Duration::from_secs(1))
            .unwrap_or_else(|_| panic!("failed to read from telnet connection",));

        //let mut message = "".to_string();
        if let TelnetEvent::Data(data) = event {
            // return this data as a string
            String::from_utf8(data.to_vec()).expect("server sent invalid UTF-8 string")
        } else {
            panic!(
                "did not receive expected DATA event, got this instead: {:?}",
                event
            );
        }
    }

    pub fn recv_contains(&mut self, text: &str) {
        let message = self.recv();
        assert!(
            message.contains(text),
            "did not find '{}' in message {:?}",
            text,
            message,
        )
    }

    pub fn recv_contains_all(&mut self, msgs: Vec<&str>) {
        let message = self.recv();
        for text in msgs.iter() {
            assert!(
                message.contains(text),
                "did not find '{}' in message {:?}",
                text,
                message,
            )
        }
    }
    pub fn recv_contains_none(&mut self, msgs: Vec<&str>) {
        let message = self.recv();
        for text in msgs.iter() {
            assert!(
                !message.contains(text),
                "found unwanted '{}' in message {:?}",
                text,
                message,
            )
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
        self.info("-------- create user -------");
        self.recv_contains("Name?");
        self.send(name);
        self.recv_contains("Password?");
        self.send(password);
        self.recv_contains("Verify?");
        self.send(password);
        self.recv_contains("Welcome to City Six.");
        self.recv(); // ignore the look that happens when we log in
        self.recv_prompt();
    }

    pub fn info(&mut self, text: &str) {
        if !text.is_empty() {
            tracing::info!("---------- {} ----------", text);
        }
    }

    pub fn test<S1: ToString, S2: ToString, S3: ToString>(
        &mut self,
        scenario: S1,
        command: S2,
        response_contains: Vec<S3>,
    ) {
        self.validate(scenario, command, Validate::Includes(response_contains));
    }

    pub fn test_exclude<S1: ToString, S2: ToString, S3: ToString>(
        &mut self,
        scenario: S1,
        command: S2,
        response_excludes: Vec<S3>,
    ) {
        self.validate(scenario, command, Validate::Excludes(response_excludes));
    }

    fn validate<S1: ToString, S2: ToString, S3: ToString>(
        &mut self,
        scenario: S1,
        command: S2,
        validate: Validate<S3>,
    ) {
        self.info(scenario.to_string().as_str());
        self.send(command.to_string().as_str());

        let (is_include, items) = match validate {
            Validate::Includes(items) => (true, items),
            Validate::Excludes(items) => (false, items),
        };

        let owned = items.into_iter().map(|s| s.to_string()).collect_vec();
        if is_include {
            self.recv_contains_all(owned.iter().map(|s| s.as_str()).collect_vec());
        } else {
            self.recv_contains_none(owned.iter().map(|s| s.as_str()).collect_vec());
        }
        self.recv_prompt();
    }
}

enum Validate<S: ToString> {
    Includes(Vec<S>),
    Excludes(Vec<S>),
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
