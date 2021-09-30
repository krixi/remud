use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use warp::Filter;

use crate::{
    engine::db::AuthDb,
    web::{
        auth::{verify_access, SCOPE_SCRIPTS},
        InternalError, JsonEmpty, Player, ScriptsRequest, ScriptsResponse, WebMessage,
    },
    world::scripting,
};

pub fn script_filters<DB>(
    db: DB,
    tx: mpsc::Sender<WebMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("scripts").and(warp::post()).and(
        create(db.clone(), tx.clone())
            .or(read_all(db.clone(), tx.clone()))
            .or(read(db.clone(), tx.clone()))
            .or(update(db.clone(), tx.clone()))
            .or(delete(db, tx)),
    )
}

#[derive(Debug, Deserialize)]
pub struct JsonScriptName {
    pub name: String,
}

impl JsonScriptName {
    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }
}

fn json_script_name() -> impl Filter<Extract = (JsonScriptName,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

#[derive(Debug, Deserialize)]
pub struct JsonScript {
    pub name: String,
    pub trigger: String,
    pub code: String,
}

impl JsonScript {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

fn json_script() -> impl Filter<Extract = (JsonScript,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 1024).and(warp::body::json())
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
        let (name, trigger, code) = script.into_parts();

        JsonScriptResponse {
            name: name.into_string(),
            trigger: trigger.to_string(),
            code,
            error: error.map(|e| e.into()),
        }
    }
}

#[derive(Debug, Serialize)]
struct JsonScriptsResponse {
    scripts: Vec<JsonScriptInfo>,
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
        let (name, trigger, code) = script.into_parts();

        JsonScriptInfo {
            name: name.into_string(),
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

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("bad trigger name")]
    BadTrigger,
    #[error("bad script name")]
    BadScriptName,
    #[error("duplicate script found")]
    DuplicateName,
    #[error("script not found")]
    ScriptNotFound,
}

impl warp::reject::Reject for ScriptError {}

pub fn create<DB>(
    db: DB,
    tx: mpsc::Sender<WebMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("create")
        .and(verify_access(db, vec![SCOPE_SCRIPTS.to_string()]))
        .and(json_script())
        .and(with_sender(tx))
        .and_then(handle_create)
}

pub fn read<DB>(
    db: DB,
    tx: mpsc::Sender<WebMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("read")
        .and(verify_access(db, vec![SCOPE_SCRIPTS.to_string()]))
        .and(json_script_name())
        .and(with_sender(tx))
        .and_then(handle_read)
}

pub fn read_all<DB>(
    db: DB,
    tx: mpsc::Sender<WebMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("read")
        .and(warp::path("all"))
        .and(verify_access(db, vec![SCOPE_SCRIPTS.to_string()]))
        .and(with_sender(tx))
        .and_then(handle_read_all)
}

pub fn update<DB>(
    db: DB,
    tx: mpsc::Sender<WebMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("update")
        .and(verify_access(db, vec![SCOPE_SCRIPTS.to_string()]))
        .and(json_script())
        .and(with_sender(tx))
        .and_then(handle_update)
}

pub fn delete<DB>(
    db: DB,
    tx: mpsc::Sender<WebMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("delete")
        .and(verify_access(db, vec![SCOPE_SCRIPTS.to_string()]))
        .and(json_script_name())
        .and(with_sender(tx))
        .and_then(handle_delete)
}

#[tracing::instrument(
    name = "create script",
    skip_all,
    fields(
        player = player.name.as_str(),
        script = script.name.as_str()
    )
)]
async fn handle_create(
    player: Player,
    script: JsonScript,
    sender: mpsc::Sender<WebMessage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    tracing::info!("player {} creating script {}", player.name(), script.name());

    let (tx, rx) = oneshot::channel();
    if let Err(err) = sender
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::CreateScript(script),
        })
        .await
    {
        tracing::error!("failed to dispatch CreateScript to engine: {}", err);
        return Err(warp::reject::custom(InternalError {}));
    };

    match rx.await {
        Ok(ScriptsResponse::ScriptCompiled(error)) => {
            Ok(warp::reply::json(&CompileResponse { error }))
        }
        Ok(ScriptsResponse::Error(e)) => Err(warp::reject::custom(e)),
        other => {
            tracing::error!("received unexpected response to CreateScript: {:?}", other);
            Err(warp::reject::custom(InternalError {}))
        }
    }
}

#[tracing::instrument(
    name = "read script",
    skip_all,
    fields(
        player = player.name.as_str(),
        script = script_name.as_str()
    )
)]
async fn handle_read(
    player: Player,
    script_name: JsonScriptName,
    sender: mpsc::Sender<WebMessage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    tracing::info!(
        "player {} reading script {}",
        player.name(),
        script_name.as_str()
    );
    let (tx, rx) = oneshot::channel();
    if let Err(err) = sender
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::ReadScript(script_name),
        })
        .await
    {
        tracing::error!("failed to dispatch ReadScript to engine: {}", err);
        return Err(warp::reject::custom(InternalError {}));
    };

    match rx.await {
        Ok(ScriptsResponse::Script(script)) => Ok(warp::reply::json(&script)),
        Ok(ScriptsResponse::Error(err)) => Err(warp::reject::custom(err)),
        other => {
            tracing::error!("received unexpected response to ReadScript: {:?}", other);
            Err(warp::reject::custom(InternalError {}))
        }
    }
}

#[tracing::instrument(
    name = "list scripts",
    skip_all
    fields(player = player.name.as_str())
)]
async fn handle_read_all(
    player: Player,
    sender: mpsc::Sender<WebMessage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    tracing::info!("player {} reading all scripts", player.name());

    let (tx, rx) = oneshot::channel();
    if let Err(err) = sender
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::ReadAllScripts,
        })
        .await
    {
        tracing::error!("failed to dispatch ReadScript to engine: {}", err);
        return Err(warp::reject::custom(InternalError {}));
    };

    match rx.await {
        Ok(ScriptsResponse::ScriptList(scripts)) => {
            Ok(warp::reply::json(&JsonScriptsResponse { scripts }))
        }
        Ok(ScriptsResponse::Error(err)) => Err(warp::reject::custom(err)),
        other => {
            tracing::error!("received unexpected response to ReadScript: {:?}", other);
            Err(warp::reject::custom(InternalError {}))
        }
    }
}

#[tracing::instrument(
    name = "update script",
    skip_all,
    fields(
        player = player.name.as_str(),
        script = script.name.as_str()
    )
)]
async fn handle_update(
    player: Player,
    script: JsonScript,
    sender: mpsc::Sender<WebMessage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    tracing::info!("player {} updating script {}", player.name(), script.name());

    let (tx, rx) = oneshot::channel();
    if let Err(err) = sender
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::UpdateScript(script),
        })
        .await
    {
        tracing::error!("failed to dispatch UpdateScript to engine: {}", err);
        return Err(warp::reject::custom(InternalError {}));
    };

    match rx.await {
        Ok(ScriptsResponse::ScriptCompiled(error)) => {
            Ok(warp::reply::json(&CompileResponse { error }))
        }
        Ok(ScriptsResponse::Error(err)) => Err(warp::reject::custom(err)),
        other => {
            tracing::error!("received unexpected response to ReadScript: {:?}", other);
            Err(warp::reject::custom(InternalError {}))
        }
    }
}

#[tracing::instrument(
    name = "delete script",
    skip_all,
    fields(
        player = player.name.as_str(),
        script = script_name.as_str()
    )
)]
async fn handle_delete(
    player: Player,
    script_name: JsonScriptName,
    sender: mpsc::Sender<WebMessage>,
) -> Result<impl warp::Reply, warp::Rejection> {
    tracing::info!(
        "player {} deleting script {}",
        player.name(),
        script_name.as_str()
    );

    let (tx, rx) = oneshot::channel();
    if let Err(err) = sender
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::DeleteScript(script_name),
        })
        .await
    {
        tracing::error!("failed to dispatch DeleteScript to engine: {}", err);
        return Err(warp::reject::custom(InternalError {}));
    };

    match rx.await {
        Ok(ScriptsResponse::Done) => Ok(warp::reply::json(&JsonEmpty {})),
        Ok(ScriptsResponse::Error(err)) => Err(warp::reject::custom(err)),
        other => {
            tracing::error!("received unexpected response to DeleteScript: {:?}", other);
            Err(warp::reject::custom(InternalError {}))
        }
    }
}

fn with_sender(
    tx: mpsc::Sender<WebMessage>,
) -> impl Filter<Extract = (mpsc::Sender<WebMessage>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || tx.clone())
}
