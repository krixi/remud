use serde::{Deserialize, Serialize};
use tide::{
    http::{headers::HeaderValue, mime},
    security::{CorsMiddleware, Origin},
    Body, Request, Response, Server, StatusCode,
};
use tide_http_auth::{Authentication, BearerAuthScheme};
use tokio::sync::oneshot;

use crate::{
    engine::db::AuthDb,
    web::{Context, Player, ScriptsRequest, ScriptsResponse, WebMessage},
    world::scripting,
};

pub fn scripts_endpoint<DB>(context: Context<DB>) -> Server<Context<DB>>
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    let mut scripts = tide::with_state(context);

    let cors = CorsMiddleware::new()
        .allow_methods("POST".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"));

    scripts.with(cors);
    scripts.with(Authentication::new(BearerAuthScheme::default()));
    scripts.at("/create").post(create_script);
    scripts.at("/read").post(read_script);
    scripts.at("/read/all").post(read_all_scripts);
    scripts.at("/update").post(update_script);
    scripts.at("/delete").post(delete_script);

    scripts
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

async fn create_script<DB: AuthDb>(mut req: Request<Context<DB>>) -> tide::Result {
    let player = if let Some(player) = req.ext::<Player>() {
        player
    } else {
        return Ok(Response::new(StatusCode::Unauthorized));
    };

    if !player.access.contains(&"scripts".to_string()) {
        return Ok(Response::new(StatusCode::Unauthorized));
    }

    let script = req.body_json::<JsonScript>().await?;
    tracing::debug!("Create script: {:?}", script);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::CreateScript(script),
        })
        .await?;

    match rx.await? {
        ScriptsResponse::ScriptCompiled(error) => Ok(Response::builder(200)
            .body(Body::from_json(&CompileResponse { error })?)
            .content_type(mime::JSON)
            .build()),
        ScriptsResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn read_script<DB: AuthDb>(mut req: Request<Context<DB>>) -> tide::Result {
    let player = if let Some(player) = req.ext::<Player>() {
        player
    } else {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    };

    if !player.access.contains(&"scripts".to_string()) {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    }

    let name = req.body_json::<JsonScriptName>().await?;
    tracing::debug!("Read script: {:?}", name);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::ReadScript(name),
        })
        .await?;

    match rx.await? {
        ScriptsResponse::Script(script) => Ok(Response::builder(200)
            .body(Body::from_json(&script)?)
            .content_type(mime::JSON)
            .build()),
        ScriptsResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn read_all_scripts<DB: AuthDb>(req: Request<Context<DB>>) -> tide::Result {
    let player = if let Some(player) = req.ext::<Player>() {
        player
    } else {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    };

    if !player.access.contains(&"scripts".to_string()) {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    }

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::ReadAllScripts,
        })
        .await?;

    match rx.await? {
        ScriptsResponse::ScriptList(scripts) => Ok(Response::builder(200)
            .body(Body::from_json(&JsonScripts { scripts })?)
            .content_type(mime::JSON)
            .build()),
        ScriptsResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn update_script<DB: AuthDb>(mut req: Request<Context<DB>>) -> tide::Result {
    let player = if let Some(player) = req.ext::<Player>() {
        player
    } else {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    };

    if !player.access.contains(&"scripts".to_string()) {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    }

    let script = req.body_json::<JsonScript>().await?;
    tracing::debug!("Update script: {:?}", script);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::UpdateScript(script),
        })
        .await?;

    match rx.await? {
        ScriptsResponse::ScriptCompiled(error) => Ok(Response::builder(200)
            .body(Body::from_json(&CompileResponse { error })?)
            .content_type(mime::JSON)
            .build()),
        ScriptsResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}

async fn delete_script<DB: AuthDb>(mut req: Request<Context<DB>>) -> tide::Result {
    let player = if let Some(player) = req.ext::<Player>() {
        player
    } else {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    };

    if !player.access.contains(&"scripts".to_string()) {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        return Ok(response);
    }

    let name = req.body_json::<JsonScriptName>().await?;
    tracing::debug!("Delete script: {:?}", name);

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: ScriptsRequest::DeleteScript(name),
        })
        .await?;

    match rx.await? {
        ScriptsResponse::Done => Ok(Response::builder(200)
            .body("{}")
            .content_type(mime::JSON)
            .build()),
        ScriptsResponse::Error(e) => Ok(Response::new(e.status())),
        _ => Ok(Response::new(500)),
    }
}
