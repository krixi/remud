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
    CreateScript(JsonScript),
    ReadScript(JsonScriptName),
    ReadAllScripts,
    UpdateScript(JsonScript),
    DeleteScript(JsonScriptName),
}

pub enum WebResponse {
    Done,
    Error(Error),
    Script(JsonScriptResponse),
    ScriptCompiled(Option<JsonParseError>),
    ScriptList(Vec<JsonScriptInfo>),
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
pub struct JsonScriptName {
    pub name: String,
}

#[derive(Debug, Serialize)]
struct JsonScripts {
    scripts: Vec<JsonScriptInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonScript {
    pub name: String,
    pub trigger: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct JsonScriptResponse {
    pub name: String,
    pub trigger: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonParseError>,
}

impl JsonScriptResponse {
    pub fn new(script: scripting::Script, error: Option<rhai::ParseError>) -> Self {
        let scripting::Script {
            name,
            trigger,
            code,
        } = script;

        JsonScriptResponse {
            name: name.to_string(),
            trigger: trigger.to_string(),
            code,
            error: error.map(|e| e.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JsonScriptInfo {
    pub name: String,
    pub trigger: String,
    pub lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonParseError>,
}

impl JsonScriptInfo {
    pub fn new(script: scripting::Script, error: Option<rhai::ParseError>) -> Self {
        let scripting::Script {
            name,
            trigger,
            code,
        } = script;

        JsonScriptInfo {
            name: name.to_string(),
            trigger: trigger.to_string(),
            lines: code.lines().count(),
            error: error.map(|e| e.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CompileResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonParseError>,
}

#[derive(Debug, Serialize)]
pub struct JsonParseError {
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<usize>,
    message: String,
}

impl From<rhai::ParseError> for JsonParseError {
    fn from(value: rhai::ParseError) -> Self {
        JsonParseError {
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

pub fn build_web_server() -> (tide::Server<()>, mpsc::Receiver<WebMessage>) {
    let (tx, rx) = mpsc::channel(16);

    let cors = CorsMiddleware::new()
        .allow_methods("POST".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"));

    let context = Context { tx };

    let mut scripts = tide::with_state(context);
    scripts.with(cors);
    scripts.at("/create").post(create_script);
    scripts.at("/read").post(read_script);
    scripts.at("/read/all").post(read_all_scripts);
    scripts.at("/update").post(update_script);
    scripts.at("/delete").post(delete_script);

    let mut app = tide::new();
    app.at("/scripts").nest(scripts);
    app.at("/docs").serve_dir("./docs/public").unwrap();
    app.at("/admin").serve_dir("./web-client/build").unwrap();

    (app, rx)
}

async fn create_script(mut req: Request<Context>) -> tide::Result {
    let script = req.body_json::<JsonScript>().await?;
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
    let name = req.body_json::<JsonScriptName>().await?;
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
            .body(Body::from_json(&JsonScripts { scripts })?)
            .content_type(mime::JSON)
            .build()),
        WebResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn update_script(mut req: Request<Context>) -> tide::Result {
    let script = req.body_json::<JsonScript>().await?;
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
    let name = req.body_json::<JsonScriptName>().await?;
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
