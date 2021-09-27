mod auth;
pub mod scripts;

use std::{convert::Infallible, fmt};

use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use warp::{
    http::HeaderValue,
    hyper::{
        header::{CONTENT_TYPE, WWW_AUTHENTICATE},
        Response, StatusCode,
    },
    reject::Reject,
    Filter,
};

use crate::{
    engine::db::AuthDb,
    web::{
        auth::{auth_filters, AuthError},
        scripts::{
            script_filters, JsonParseError, JsonScript, JsonScriptInfo, JsonScriptName,
            JsonScriptResponse, ScriptError,
        },
    },
};

pub fn build_web_server<DB>(
    db: DB,
) -> (
    warp::Server<impl Filter<Extract = impl warp::Reply, Error = Infallible> + Clone>,
    mpsc::Receiver<WebMessage>,
)
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    let (tx, rx) = mpsc::channel(16);

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["POST", "OPTIONS"])
        .allow_headers(vec!["content-type", "x-requested-with", "authorization"]);
    let routes = auth_filters(db.clone()).or(script_filters(db, tx));
    let wrapped = routes.with(cors).recover(handle_rejection);

    (warp::serve(wrapped), rx)
}

#[derive(Debug)]
pub struct InternalError {}
impl Reject for InternalError {}

#[derive(Debug, Serialize)]
pub struct JsonEmpty {}

fn with_db<DB>(db: DB) -> impl Filter<Extract = (DB,), Error = std::convert::Infallible> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::any().map(move || db.clone())
}

#[derive(Debug)]
pub struct Player {
    name: String,
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

#[derive(Debug)]
pub enum ScriptsResponse {
    Done,
    Error(ScriptError),
    Script(JsonScriptResponse),
    ScriptCompiled(Option<JsonParseError>),
    ScriptList(Vec<JsonScriptInfo>),
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let code;
    let message;
    let mut headers = vec![(CONTENT_TYPE, HeaderValue::from_static("application/json"))];

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(err) = err.find::<ScriptError>() {
        match err {
            ScriptError::BadTrigger => {
                code = StatusCode::BAD_REQUEST;
                message = "BAD_TRIGGER";
            }
            ScriptError::BadScriptName => {
                code = StatusCode::BAD_REQUEST;
                message = "BAD_SCRIPT_NAME";
            }
            ScriptError::DuplicateName => {
                code = StatusCode::CONFLICT;
                message = "DUPLICATE_SCRIPT_NAME";
            }
            ScriptError::ScriptNotFound => {
                code = StatusCode::NOT_FOUND;
                message = "SCRIPT_NOT_FOUND";
            }
        }
    } else if let Some(err) = err.find::<AuthError>() {
        headers.push((
            WWW_AUTHENTICATE,
            HeaderValue::from_static(r#"Bearer realm="remud""#),
        ));
        match err {
            AuthError::AuthenticationError => {
                code = StatusCode::UNAUTHORIZED;
                message = "BAD_USERNAME_OR_PASSWORD";
            }
            AuthError::InvalidAuthHeader => {
                code = StatusCode::UNAUTHORIZED;
                message = "BAD_AUTH_HEADER";
            }
            AuthError::InadequateAccess => {
                code = StatusCode::UNAUTHORIZED;
                message = "INADEQUATE_ACCESS";
            }
            AuthError::InvalidToken | AuthError::VerificationError => {
                code = StatusCode::UNAUTHORIZED;
                message = "UNAUTHORIZED";
            }
        }
    } else if err.find::<InternalError>().is_some() {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    } else {
        tracing::error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let message = ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    };

    let mut response = Response::builder()
        .body(serde_json::to_vec(&message).unwrap())
        .unwrap();

    *response.status_mut() = code;

    for (key, value) in headers {
        response.headers_mut().insert(key, value);
    }

    Ok(response)
}
