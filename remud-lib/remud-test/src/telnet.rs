use std::{borrow::Cow, collections::VecDeque, time::Duration};

use ::telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};

use itertools::Itertools;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum TelnetRequest {
    Recv,
    Send(String),
}

/// TelnetConnection maintains a scratch buffer for holding incoming data and an output list which consists
/// of lines between one prompt and another. When a prompt is found in the received output, deserializing
/// into the output list is halted and the output is made available for inspection either line-by-line or
/// wholesale using one of the test* functions.
///
/// A prompt leading the buffer must be consumed before more data can be read. The send method automatically
/// consumes the last prompt - it assumes you are a good citizen and only enter commands when prompted.
/// It can be manually consumed with consume_prompt.
pub struct TelnetConnection {
    buffer: String,
    output: VecDeque<String>,
    req_tx: mpsc::Sender<TelnetRequest>,
    event_rx: mpsc::Receiver<TelnetEvent>,
}

impl TelnetConnection {
    /// Creates a new client and performs initial Telnet options negotiation.
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

    /// Prints an info line to the logs
    pub fn info<'a, S>(&mut self, text: S)
    where
        S: Into<Cow<'a, str>>,
    {
        let text = text.into();
        if !text.is_empty() {
            tracing::info!("---------- {} ----------", text);
        }
    }

    /// Consume the current prompt and dispatch a message to the server
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

    /// Consumes the next line of output, asserting if it doesn't contain text.
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

    /// Consumes the next line of output, asserting if any value in msgs is missing.
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

    /// Consumes the next line of output, asserting if any value in msgs is contained within.
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

    /// Asserts that the client is waiting at a prompt and clears it.
    /// Automatically called when using the send method.
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

    /// Asserts that the client is currently waiting at a prompt for input.
    pub async fn assert_prompt(&mut self) {
        assert!(
            self.buffer.starts_with("> "),
            "expected prompt, found: {}",
            self.buffer
        );
    }

    /// Runs a command and discards the response
    pub async fn command<'a, S1, S2>(&mut self, scenario: S1, command: S2)
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        self.info(scenario);
        self.send(command).await;

        self.recv().await;

        self.output.clear();
    }

    /// Runs a command and tests its output against the matcher
    pub async fn test_matches<'a, S1, S2>(
        &mut self,
        scenario: S1,
        command: S2,
        matcher: Matcher<'a>,
    ) where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        self.info(scenario);
        self.send(command).await;

        self.recv().await;

        let mut output = VecDeque::new();
        std::mem::swap(&mut output, &mut self.output);

        let matched = match matcher.clone() {
            Matcher::Exact(matchers) => {
                assert!(
                    matchers.len() == output.len(),
                    "wanted exact match, found different line counts for matcher and output: \
                     {:?}, {:?}",
                    matcher,
                    output,
                );

                let mut lines = output.iter();
                matchers.into_iter().all(|m| match m {
                    Match::Include(value) => lines.next().unwrap().contains(value.as_ref()),
                    Match::Exclude(value) => !lines.next().unwrap().contains(value.as_ref()),
                    Match::None => {
                        lines.next();
                        true
                    }
                })
            }
            Matcher::Ordered(matchers) => {
                let mut lines = output.iter();
                matchers.into_iter().all(|m| match m {
                    Match::Include(value) => {
                        while let Some(line) = lines.next() {
                            if line.contains(value.as_ref()) {
                                return true;
                            }
                        }
                        false
                    }
                    Match::Exclude(_) => unreachable!("invalid matcher state"),
                    Match::None => true,
                })
            }
            Matcher::Unordered(matchers) => matchers.into_iter().all(|m| match m {
                Match::Include(value) => output.iter().any(|l| l.contains(value.as_ref())),
                Match::Exclude(value) => output.iter().all(|l| !l.contains(value.as_ref())),
                Match::None => true,
            }),
        };

        assert!(
            matched,
            "matcher does not match output: {:?}, {:?}",
            matcher, output
        );
    }

    /// Runs a command and consumes all output between the command and the next prompt.
    /// Asserts if any string in response_contains does not appear in output.
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
        let includes = response_contains.into_iter().map(Into::into).collect_vec();

        self.test_matches(scenario, command, Matcher::includes(includes))
            .await;
    }

    /// Runs a command and consumes all output between the command and the next prompt.
    /// Asserts if any string in response_excludes appears in output.
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
        let excludes = response_excludes.into_iter().map(Into::into).collect_vec();

        self.test_matches(scenario, command, Matcher::excludes(excludes))
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

#[derive(Debug, Clone)]
pub enum Matcher<'a> {
    Exact(Vec<Match<'a>>),
    Ordered(Vec<Match<'a>>),
    Unordered(Vec<Match<'a>>),
}

impl<'a> Matcher<'a> {
    /// Checks that each matcher matches exactly one output line, in order
    pub fn exact(matches: impl IntoIterator<Item = Match<'a>>) -> Self {
        Self::Exact(matches.into_iter().collect_vec())
    }

    pub fn exact_includes<S>(matches: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self::Exact(
            matches
                .into_iter()
                .map(|m| Match::Include(m.into()))
                .collect_vec(),
        )
    }

    /// Checks that each matcher is present in output, in order
    pub fn ordered<S>(matches: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self::Ordered(
            matches
                .into_iter()
                .map(|m| Match::Include(m.into()))
                .collect_vec(),
        )
    }

    /// Checks matchers against output, unordered
    pub fn unordered(matches: impl IntoIterator<Item = Match<'a>>) -> Self {
        Self::Unordered(matches.into_iter().collect_vec())
    }

    /// Checks that all of the matchers are present in output
    pub fn includes<S>(matches: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self::Unordered(
            matches
                .into_iter()
                .map(|m| Match::Include(m.into()))
                .collect_vec(),
        )
    }

    /// Checks that none of the matchers are present in output
    pub fn excludes<S>(matches: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self::Unordered(
            matches
                .into_iter()
                .map(|m| Match::Exclude(m.into()))
                .collect_vec(),
        )
    }
}

#[derive(Debug, Clone)]
pub enum Match<'a> {
    Include(Cow<'a, str>),
    Exclude(Cow<'a, str>),
    None,
}

impl<'a> Match<'a> {
    pub fn include<S>(value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Match::Include(value.into())
    }

    pub fn exclude<S>(value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Match::Exclude(value.into())
    }
}
