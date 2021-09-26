mod auth;
pub mod scripts;

use std::fmt;

use thiserror::Error;
use tide::StatusCode;
use tokio::sync::{mpsc, oneshot};

use crate::{
    engine::db::AuthDb,
    web::{
        auth::auth_endpoint,
        scripts::{
            scripts_endpoint, JsonParseError, JsonScript, JsonScriptInfo, JsonScriptName,
            JsonScriptResponse,
        },
    },
};

pub fn build_web_server<DB>(db: DB) -> (tide::Server<()>, mpsc::Receiver<WebMessage>)
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    let (tx, rx) = mpsc::channel(16);

    let context = Context { db, tx };

    let mut app = tide::new();
    app.at("/auth").nest(auth_endpoint(context.clone()));
    app.at("/scripts").nest(scripts_endpoint(context));
    app.at("/docs")
        .serve_dir("./docs/public")
        .unwrap_or_else(|_| tracing::warn!("can't find ./docs/public"));
    app.at("/admin")
        .serve_dir("./web-client/build")
        .unwrap_or_else(|_| tracing::warn!("can't find ./web-client/build"));

    (app, rx)
}

#[derive(Clone)]
pub struct Context<DB: AuthDb> {
    db: DB,
    tx: mpsc::Sender<WebMessage>,
}

struct Player {
    name: String,
    access: Vec<String>,
}

pub struct WebMessage {
    pub response: oneshot::Sender<ScriptsResponse>,
    pub request: ScriptsRequest,
}

impl fmt::Debug for WebMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebMessage")
            .field("request", &self.request)
            .finish()
    }
}

#[derive(Debug)]
pub enum ScriptsRequest {
    CreateScript(JsonScript),
    ReadScript(JsonScriptName),
    ReadAllScripts,
    UpdateScript(JsonScript),
    DeleteScript(JsonScriptName),
}

pub enum ScriptsResponse {
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
