use std::fmt;

use crate::world::scripting;
use serde::{Deserialize, Serialize};
use tide::{http::mime, Body, Request, Response};
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
    AllScripts(Vec<Script>),
    Done,
    Error,
    Script(Script),
}

#[derive(Debug, Serialize)]
struct Scripts {
    scripts: Vec<Script>,
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

#[derive(Debug, Deserialize)]
pub struct ScriptName {
    pub name: String,
}

#[derive(Clone)]
pub struct Context {
    pub tx: mpsc::Sender<WebMessage>,
}

pub fn build_web_server() -> (tide::Server<Context>, mpsc::Receiver<WebMessage>) {
    let (tx, rx) = mpsc::channel(16);

    let mut app = tide::with_state(Context { tx });
    app.at("/scripts/create").post(create_script);
    app.at("/scripts/read").post(read_script);
    app.at("/scripts/read/all").post(read_all_scripts);
    app.at("/scripts/update").post(update_script);
    app.at("/scripts/delete").post(delete_script);

    (app, rx)
}

async fn create_script(mut req: Request<Context>) -> tide::Result {
    let script = req.body_json::<Script>().await?;

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::CreateScript(script),
        })
        .await?;

    if let WebResponse::Done = rx.await? {
        Ok(Response::builder(200)
            .body("{}")
            .content_type(mime::JSON)
            .build())
    } else {
        Ok(Response::new(500))
    }
}

async fn read_script(mut req: Request<Context>) -> tide::Result {
    let name = req.body_json::<ScriptName>().await?;

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::ReadScript(name),
        })
        .await?;

    match rx.await {
        Ok(response) => {
            if let WebResponse::Script(script) = response {
                Ok(Response::builder(200)
                    .body(Body::from_json(&script)?)
                    .content_type(mime::JSON)
                    .build())
            } else {
                Ok(Response::new(500))
            }
        }
        Err(_) => Ok(Response::new(500)),
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

    match rx.await {
        Ok(response) => {
            if let WebResponse::AllScripts(scripts) = response {
                Ok(Response::builder(200)
                    .body(Body::from_json(&Scripts { scripts })?)
                    .content_type(mime::JSON)
                    .build())
            } else {
                Ok(Response::new(500))
            }
        }
        Err(_) => Ok(Response::new(500)),
    }
}

async fn update_script(mut req: Request<Context>) -> tide::Result {
    let script = req.body_json::<Script>().await?;

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::UpdateScript(script),
        })
        .await?;

    if let WebResponse::Done = rx.await? {
        Ok(Response::builder(200)
            .body("{}")
            .content_type(mime::JSON)
            .build())
    } else {
        Ok(Response::new(500))
    }
}

async fn delete_script(mut req: Request<Context>) -> tide::Result {
    let name = req.body_json::<ScriptName>().await?;

    let (tx, rx) = oneshot::channel();
    req.state()
        .tx
        .send(WebMessage {
            response: tx,
            request: WebRequest::DeleteScript(name),
        })
        .await?;

    if let WebResponse::Done = rx.await? {
        Ok(Response::builder(200)
            .body("{}")
            .content_type(mime::JSON)
            .build())
    } else {
        Ok(Response::new(500))
    }
}
