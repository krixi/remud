use std::fmt;

use crate::world::scripting;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tide::{
    http::{headers::HeaderValue, mime},
    security::{CorsMiddleware, Origin},
    Body, Request, Response, StatusCode,
};
use tokio::sync::{mpsc, oneshot};

pub struct WebMessage {
    pub response: oneshot::Sender<WebResponse>,
    pub request: WebRequest,
}

impl fmt::Debug for WebMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebMessage")
            .field("request", &self.request)
            .finish()
    }
}

#[derive(Debug)]
pub enum WebRequest {
    CreateScript(Script),
    ReadScript(ScriptName),
    ReadAllScripts,
    UpdateScript(Script),
    DeleteScript(ScriptName),
}

pub enum WebResponse {
    Done,
    Error(Error),
    Script(Script),
    ScriptCompiled(Option<ParseError>),
    ScriptList(Vec<ScriptInfo>),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid trigger")]
    BadTrigger,
    #[error("invalid script name")]
    BadScriptName,
    #[error("duplicate name")]
    DuplicateName,
    #[error("script not found")]
    ScriptNotFound,
}

impl Error {
    fn status(&self) -> StatusCode {
        match self {
            Error::BadTrigger => StatusCode::BadRequest,
            Error::BadScriptName => StatusCode::BadRequest,
            Error::DuplicateName => StatusCode::Conflict,
            Error::ScriptNotFound => StatusCode::NotFound,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ScriptName {
    pub name: String,
}

#[derive(Debug, Serialize)]
struct Scripts {
    scripts: Vec<ScriptInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Script {
    pub name: String,
    pub trigger: String,
    pub code: String,
}

impl From<scripting::Script> for Script {
    fn from(value: scripting::Script) -> Self {
        let scripting::Script {
            name,
            trigger,
            code,
        } = value;

        Script {
            name: name.to_string(),
            trigger: trigger.to_string(),
            code,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ScriptInfo {
    pub name: String,
    pub trigger: String,
    pub lines: usize,
}

impl From<scripting::Script> for ScriptInfo {
    fn from(value: scripting::Script) -> Self {
        let scripting::Script {
            name,
            trigger,
            code,
        } = value;

        ScriptInfo {
            name: name.to_string(),
            trigger: trigger.to_string(),
            lines: code.lines().count(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CompileResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ParseError>,
}

#[derive(Debug, Serialize)]
pub struct ParseError {
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<usize>,
    message: String,
}

impl From<rhai::ParseError> for ParseError {
    fn from(value: rhai::ParseError) -> Self {
        ParseError {
            line: value.1.line(),
            position: value.1.position(),
            message: value.0.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Context {
    pub tx: mpsc::Sender<WebMessage>,
}

pub fn build_web_server() -> (tide::Server<Context>, mpsc::Receiver<WebMessage>) {
    let (tx, rx) = mpsc::channel(16);

    let cors = CorsMiddleware::new()
        .allow_methods("POST".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"));

    let mut app = tide::with_state(Context { tx });
    app.with(cors);
    app.at("/scripts/create").post(create_script);
    app.at("/scripts/read").post(read_script);
    app.at("/scripts/read/all").post(read_all_scripts);
    app.at("/scripts/update").post(update_script);
    app.at("/scripts/delete").post(delete_script);

    app.at("/docs").serve_dir("./docs/public").unwrap();
    app.at("/admin").serve_dir("./web-client/build").unwrap();

    (app, rx)
}

async fn create_script(mut req: Request<Context>) -> tide::Result {
    let script = req.body_json::<Script>().await?;
    tracing::debug!("Create script: {:?}", script);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::CreateScript(script),
        })
        .await?;

    match rx.await? {
        WebResponse::ScriptCompiled(error) => Ok(Response::builder(200)
            .body(Body::from_json(&CompileResponse { error })?)
            .content_type(mime::JSON)
            .build()),
        WebResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn read_script(mut req: Request<Context>) -> tide::Result {
    let name = req.body_json::<ScriptName>().await?;
    tracing::debug!("Read script: {:?}", name);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::ReadScript(name),
        })
        .await?;

    match rx.await? {
        WebResponse::Script(script) => Ok(Response::builder(200)
            .body(Body::from_json(&script)?)
            .content_type(mime::JSON)
            .build()),
        WebResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn read_all_scripts(req: Request<Context>) -> tide::Result {
    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::ReadAllScripts,
        })
        .await?;

    match rx.await? {
        WebResponse::ScriptList(scripts) => Ok(Response::builder(200)
            .body(Body::from_json(&Scripts { scripts })?)
            .content_type(mime::JSON)
            .build()),
        WebResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn update_script(mut req: Request<Context>) -> tide::Result {
    let script = req.body_json::<Script>().await?;
    tracing::debug!("Update script: {:?}", script);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::UpdateScript(script),
        })
        .await?;

    match rx.await? {
        WebResponse::ScriptCompiled(error) => Ok(Response::builder(200)
            .body(Body::from_json(&CompileResponse { error })?)
            .content_type(mime::JSON)
            .build()),
        WebResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn delete_script(mut req: Request<Context>) -> tide::Result {
    let name = req.body_json::<ScriptName>().await?;
    tracing::debug!("Delete script: {:?}", name);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::DeleteScript(name),
        })
        .await?;

    match rx.await? {
        WebResponse::Done => Ok(Response::builder(200)
            .body("{}")
            .content_type(mime::JSON)
            .build()),
        WebResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}
