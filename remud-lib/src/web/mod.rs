mod auth;
pub mod scripts;
mod security;
pub mod ws;

use std::{convert::Infallible, fmt, path::Path};

use serde::Serialize;
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use warp::{
    any,
    http::HeaderValue,
    hyper::{
        header::{CONTENT_TYPE, WWW_AUTHENTICATE},
        Response, StatusCode,
    },
    reject::Reject,
    serve, Filter, Rejection, Reply, Server, TlsServer,
};

use crate::web::ws::websocket_filters;
use crate::{
    engine::{db::AuthDb, ClientMessage},
    web::{
        auth::{auth_filters, AuthError},
        scripts::{
            script_filters, JsonParseError, JsonScript, JsonScriptInfo, JsonScriptName,
            JsonScriptResponse, ScriptError,
        },
        security::{retrieve_certificate, retrieve_jwt_key, CertificateError, JwtError},
    },
};

#[derive(Debug)]
pub struct WebOptions<'a> {
    port: u16,
    keys: &'a Path,
    cors: Vec<&'a str>,
    tls: Option<TlsOptions<'a>>,
}

impl<'a> WebOptions<'a> {
    pub fn new(port: u16, keys: &'a Path, cors: Vec<&'a str>, tls: Option<TlsOptions<'a>>) -> Self {
        WebOptions {
            port,
            keys,
            cors,
            tls,
        }
    }

    pub fn uri(&self) -> String {
        if let Some(TlsOptions { domain, .. }) = &self.tls {
            format!("https://{}:{}", domain, self.port)
        } else {
            format!("https://0.0.0.0:{}", self.port)
        }
    }

    pub fn cors(&self) -> &[&str] {
        self.cors.as_slice()
    }

    fn address(&self) -> ([u8; 4], u16) {
        ([0, 0, 0, 0], self.port)
    }
}

#[derive(Debug)]
pub struct TlsOptions<'a> {
    domain: &'a str,
    email: &'a str,
}

impl<'a> TlsOptions<'a> {
    pub fn new(domain: &'a str, email: &'a str) -> Self {
        TlsOptions { domain, email }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to acquire certificate: {0}")]
    CertificateError(#[from] CertificateError),
    #[error("failed to acquire JWT key: {0}")]
    JwtError(#[from] JwtError),
}

#[tracing::instrument(name = "starting web server", skip(db, web_tx, client_tx))]
pub(crate) async fn run_web_server<'a, DB>(
    options: &WebOptions<'a>,
    db: DB,
    web_tx: mpsc::Sender<WebMessage>,
    client_tx: mpsc::Sender<ClientMessage>,
) -> Result<JoinHandle<()>, Error>
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    let address = options.address();
    let handle = if let Some(tls) = &options.tls {
        let web_server = build_tls_server(
            db,
            web_tx,
            client_tx,
            options.keys,
            options.cors.as_slice(),
            tls.domain,
            tls.email,
        )
        .await?;
        tokio::spawn(async move { web_server.run(address).await })
    } else {
        let web_server =
            build_web_server(db, web_tx, client_tx, options.keys, options.cors.as_slice()).await?;
        tokio::spawn(async move { web_server.run(address).await })
    };

    Ok(handle)
}

async fn build_tls_server<DB>(
    db: DB,
    web_tx: mpsc::Sender<WebMessage>,
    client_tx: mpsc::Sender<ClientMessage>,
    key_path: &Path,
    cors: &[&str],
    domain: &str,
    email: &str,
) -> Result<TlsServer<impl Filter<Extract = impl Reply, Error = Rejection> + Clone>, Error>
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    let certificate = retrieve_certificate(key_path, domain, email).await?;

    Ok(build_web_server(db, web_tx, client_tx, key_path, cors)
        .await?
        .tls()
        .key(certificate.private_key())
        .cert(certificate.certificate()))
}

async fn build_web_server<DB>(
    db: DB,
    web_tx: mpsc::Sender<WebMessage>,
    client_tx: mpsc::Sender<ClientMessage>,
    key_path: &Path,
    cors: &[&str],
) -> Result<Server<impl Filter<Extract = impl Reply, Error = Rejection> + Clone>, Error>
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    retrieve_jwt_key(key_path).await?;

    let cors = if cors.is_empty() {
        warp::cors().allow_any_origin()
    } else {
        warp::cors().allow_origins(cors.iter().copied())
    }
    .allow_methods(vec!["POST", "OPTIONS"])
    .allow_headers(vec!["content-type", "x-requested-with", "authorization"]);

    let routes = auth_filters(db.clone())
        .or(script_filters(db, web_tx))
        .or(websocket_filters(client_tx))
        .recover(handle_rejection);
    let wrapped = routes.with(cors);

    Ok(serve(wrapped))
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
    any().map(move || db.clone())
}

#[derive(Debug)]
pub struct Player {
    name: String,
}

impl Player {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
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

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
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
