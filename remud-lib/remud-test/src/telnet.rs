use std::{borrow::Cow, collections::VecDeque, time::Duration};

use ::telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};

use itertools::Itertools;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum TelnetRequest {
    Recv,
    Send(String),
}

// TelnetConnection maintains a scratch buffer for holding incoming data and an output list which consists
// of lines between one prompt and another. When a prompt is found in the received output, deserializing
// into the output list is halted and the output is made available for inspection either line-by-line or
// wholesale using one of the test* functions.
//
// A prompt leading the buffer must be consumed before more data can be read. The send method automatically
// consumes the last prompt - it assumes you are a good citizen and only enter commands when prompted.
// It can be manually consumed with consume_prompt.
pub struct TelnetConnection {
    buffer: String,
    output: VecDeque<String>,
    req_tx: mpsc::Sender<TelnetRequest>,
    event_rx: mpsc::Receiver<TelnetEvent>,
}

impl TelnetConnection {
    // Creates a new client and performs initial Telnet options negotiation.
    pub fn new(port: u16) -> Self {
        let (req_tx, mut req_rx) = mpsc::channel(16);
        let (event_tx, event_rx) = mpsc::channel(16);

        std::thread::spawn(move || {
            // set up connection
            let mut connection =
                Telnet::connect(("127.0.0.1", port), 1024).expect("failed to connect to ReMUD");

            match connection
                .read_timeout(Duration::from_secs(10))
                .expect("did not receive DO TTYPE")
            {
                TelnetEvent::Negotiation(NegotiationAction::Do, TelnetOption::TTYPE) => {
                    connection.negotiate(NegotiationAction::Wont, TelnetOption::TTYPE)
                }
                other => panic!(
                    "received unexpected message waiting for DO TTYPE: {:?}",
                    other
                ),
            }

            while let Some(req) = req_rx.blocking_recv() {
                match req {
                    TelnetRequest::Recv => {
                        let event = connection.read_timeout(Duration::from_secs(10)).unwrap();
                        event_tx.blocking_send(event).unwrap();
                    }
                    TelnetRequest::Send(message) => {
                        connection.write(message.as_bytes()).unwrap();
                    }
                }
            }
        });

        TelnetConnection {
            buffer: String::new(),
            output: VecDeque::new(),
            req_tx,
            event_rx,
        }
    }

    pub fn info<'a, S>(&mut self, text: S)
    where
        S: Into<Cow<'a, str>>,
    {
        let text = text.into();
        if !text.is_empty() {
            tracing::info!("---------- {} ----------", text);
        }
    }

    // Consume the current prompt and dispatch a message to the server
    pub async fn send<'a, S>(&mut self, line: S)
    where
        S: Into<Cow<'a, str>>,
    {
        self.consume_prompt().await;

        // server requires \r\n to indicate command entry
        let line = line.into();
        tracing::debug!("input> {}", line.as_ref());
        let message = format!("{}\r\n", line.as_ref());

        self.req_tx
            .send(TelnetRequest::Send(message))
            .await
            .unwrap();
    }

    // Consumes the next line of output, asserting if it doesn't contain text.
    pub async fn line_contains<'a, S>(&mut self, text: S)
    where
        S: Into<Cow<'a, str>>,
    {
        let text = text.into();
        let message = self.read_line().await;
        assert!(
            message.contains(text.as_ref()),
            "did not find '{}' in message {:?}",
            text,
            message,
        )
    }

    // Consumes the next line of output, asserting if any value in msgs is missing.
    pub async fn line_contains_all<'a, S>(&mut self, msgs: Vec<S>)
    where
        S: Into<Cow<'a, str>>,
    {
        let message = self.read_line().await;
        for text in msgs {
            let text = text.into();
            assert!(
                message.contains(text.as_ref()),
                "did not find '{}' in message {:?}",
                text,
                message,
            )
        }
    }

    // Consumes the next line of output, asserting if any value in msgs is contained within.
    pub async fn line_contains_none<'a, S>(&mut self, msgs: Vec<S>)
    where
        S: Into<Cow<'a, str>>,
    {
        let message = self.read_line().await;
        for text in msgs {
            let text = text.into();
            assert!(
                !message.contains(text.as_ref()),
                "found unwanted '{}' in message {:?}",
                text,
                message,
            )
        }
    }

    // Asserts that the client is waiting at a prompt and clears it.
    // Automatically called when using the send method.
    pub async fn consume_prompt(&mut self) {
        if self.buffer.is_empty() {
            self.recv().await
        }

        self.assert_prompt().await;
        match self.buffer.strip_prefix("> ") {
            Some(rest) => self.buffer = rest.to_string(),
            None => panic!("unable to remove prompt as prefix from buffer"),
        };
        tracing::info!("consumed prompt");
    }

    // Asserts that the client is currently waiting at a prompt for input.
    pub async fn assert_prompt(&mut self) {
        assert!(
            self.buffer.starts_with("> "),
            "expected prompt, found: {}",
            self.buffer
        );
    }

    // Runs a command and consumes all output between the command and the next prompt.
    // Asserts if any string in response_contains does not appear in output.
    pub async fn test<'a, S1, S2, S3>(
        &mut self,
        scenario: S1,
        command: S2,
        response_contains: Vec<S3>,
    ) where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
        S3: Into<Cow<'a, str>>,
    {
        let includes = response_contains
            .into_iter()
            .map(|s| s.into().to_owned().to_string())
            .collect_vec();
        self.validate(scenario, command, Validate::Includes(includes))
            .await;
    }

    // Runs a command and consumes all output between the command and the next prompt.
    // Asserts if any string in response_excludes appears in output.
    pub async fn test_exclude<'a, S1, S2, S3>(
        &mut self,
        scenario: S1,
        command: S2,
        response_excludes: Vec<S3>,
    ) where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
        S3: Into<Cow<'a, str>>,
    {
        self.validate(
            scenario,
            command,
            Validate::Excludes(
                response_excludes
                    .into_iter()
                    .map(|s| s.into().to_owned().to_string())
                    .collect_vec(),
            ),
        )
        .await;
    }

    async fn recv(&mut self) {
        // receive from telnet until we receive a prompt
        // note there could be prompts in the middle, we stop when the buffer ends in a
        // prompt w/o a newline which indicates an end to a section of output.
        if self.buffer.is_empty() {
            loop {
                self.req_tx.send(TelnetRequest::Recv).await.unwrap();
                if let Some(event) = self.event_rx.recv().await {
                    if let TelnetEvent::Data(data) = event {
                        let new =
                            std::str::from_utf8(&data).expect("server sent invalid UTF-8 string");
                        self.buffer.push_str(new);
                    } else {
                        panic!("did not receive expected DATA event, got: {:?}", event);
                    }
                } else {
                    panic!("failed to read from telnet: channel closed")
                };

                if self.buffer.ends_with("> ") {
                    break;
                }
            }
        } else {
            tracing::info!("skipping recv, have output");
        }

        // split off lines into output until a prompt, keeping the rest in the buffer
        let mut next = self.buffer.as_str();
        loop {
            if next.starts_with("> ") {
                tracing::info!("found prompt - must be consumed before receiving");
                tracing::info!("next is: {}", next);
                break;
            }

            if let Some((output, rest)) = next.split_once("\r\n") {
                tracing::info!("splitting: {} <-> {}", output, rest);
                if !output.is_empty() {
                    self.output.push_back(output.trim().to_string());
                }
                next = rest;
            } else {
                break;
            }
        }
        self.buffer = next.to_string();

        tracing::info!("read result:");
        tracing::info!("output: {:?}", self.output);
        tracing::info!("buffer: {:?}", self.buffer);
    }

    // Consumes the first line of output, returning it.
    async fn read_line(&mut self) -> String {
        if self.output.is_empty() {
            self.recv().await
        }

        assert!(
            self.output.front().is_some(),
            "expected value, found no output. Did you forget to consume a prompt?"
        );
        let line = self.output.pop_front().unwrap();

        tracing::info!("reading output line: {}", line.as_str());

        line
    }

    async fn validate<'a, S1, S2>(&mut self, scenario: S1, command: S2, validate: Validate)
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        self.info(scenario);
        self.send(command).await;

        self.recv().await;

        let (is_include, items) = match validate {
            Validate::Includes(items) => (true, items),
            Validate::Excludes(items) => (false, items),
        };

        let mut output = VecDeque::new();
        std::mem::swap(&mut output, &mut self.output);

        if is_include {
            assert!(items
                .into_iter()
                .all(|i| output.iter().any(|b| b.contains(i.as_str()))));
        } else {
            assert!(!items
                .into_iter()
                .any(|i| output.iter().any(|b| b.contains(i.as_str()))));
        }
    }
}

pub struct TelnetPlayer {
    pub name: String,
    pub password: String,
    connection: TelnetConnection,
}

impl TelnetPlayer {
    pub fn new<'a, S1, S2>(connection: TelnetConnection, player: S1, password: S2) -> Self
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        TelnetPlayer {
            name: player.into().to_owned().to_string(),
            password: password.into().to_owned().to_string(),
            connection,
        }
    }
}

impl std::ops::Deref for TelnetPlayer {
    type Target = TelnetConnection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl std::ops::DerefMut for TelnetPlayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

impl TelnetPlayer {}

enum Validate {
    Includes(Vec<String>),
    Excludes(Vec<String>),
}
