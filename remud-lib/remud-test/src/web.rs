use std::{borrow::Cow, time::Duration};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Empty {}

#[derive(Debug, Serialize)]
struct JsonAuth {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct JsonAuthResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct JsonRefresh {
    refresh_token: String,
}

#[derive(Debug, strum::ToString, strum::EnumString)]
pub enum Trigger {
    Drop,
    Emote,
    Exits,
    Get,
    Init,
    Inventory,
    Look,
    LookAt,
    Move,
    Say,
    Send,
    Timer,
    Use,
}

impl serde::Serialize for Trigger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Debug, Serialize)]
pub struct JsonScript {
    name: String,
    trigger: Trigger,
    code: String,
}

impl JsonScript {
    pub fn new<'a, S1, S2>(name: S1, trigger: Trigger, code: S2) -> Self
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        JsonScript {
            name: name.into().to_owned().to_string(),
            trigger: trigger,
            code: code.into().to_owned().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JsonScriptName {
    name: String,
}

impl From<&str> for JsonScriptName {
    fn from(value: &str) -> Self {
        JsonScriptName {
            name: value.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonListResponse {
    scripts: Vec<JsonScriptInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonScriptResponse {
    pub name: String,
    pub trigger: String,
    pub code: String,
    pub error: Option<JsonErrorInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonErrorResponse {
    pub error: Option<JsonErrorInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonScriptInfo {
    pub name: String,
    pub trigger: String,
    pub lines: i64,
    pub error: Option<JsonErrorInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonErrorInfo {
    pub line: Option<i64>,
    pub position: Option<i64>,
    pub message: String,
}

#[derive(Clone)]
pub struct WebClient {
    port: u16,
    client: reqwest::Client,
}

impl WebClient {
    const URL: &'static str = "http://127.0.0.1";

    pub fn new(port: u16) -> Self {
        let client = reqwest::Client::new();
        WebClient { port, client }
    }

    pub async fn login<'a, S1, S2>(
        self,
        player: S1,
        password: S2,
    ) -> Result<AuthenticatedWebClient, StatusCode>
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        match self
            .client
            .post(format!("{}:{}/auth/login", Self::URL, self.port))
            .json(&JsonAuth {
                username: player.into().to_string(),
                password: password.into().to_string(),
            })
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let response: JsonAuthResponse = response.json().await.unwrap();
                    Ok(AuthenticatedWebClient {
                        client: self,
                        access_token: response.access_token,
                        refresh_token: response.refresh_token,
                    })
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}:{}{}", WebClient::URL, self.port, path))
    }
}

#[derive(Clone)]
pub struct AuthenticatedWebClient {
    client: WebClient,
    access_token: String,
    refresh_token: String,
}

impl AuthenticatedWebClient {
    fn post_auth(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(path)
            .timeout(Duration::from_secs(10))
            .bearer_auth(self.access_token.as_str())
    }

    pub async fn refresh_auth(&mut self) -> Result<(), StatusCode> {
        match self
            .client
            .post("/auth/refresh")
            .timeout(Duration::from_secs(10))
            .json(&JsonRefresh {
                refresh_token: self.refresh_token.clone(),
            })
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let response = response.json::<JsonAuthResponse>().await.unwrap();
                    self.access_token = response.access_token;
                    self.refresh_token = response.refresh_token;
                    Ok(())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    // Does not consume self to allow testing of code reuse
    pub async fn logout(&self) -> Result<(), StatusCode> {
        match self.post_auth("/auth/logout").json(&Empty {}).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<Empty>().await.unwrap();
                    Ok(())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub async fn create_script(
        &self,
        script: &JsonScript,
    ) -> Result<Option<JsonErrorInfo>, StatusCode> {
        match self.post_auth("/scripts/create").json(script).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response.json::<JsonErrorResponse>().await.unwrap().error)
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub async fn read_script(
        &self,
        script: &JsonScriptName,
    ) -> Result<JsonScriptResponse, StatusCode> {
        match self.post_auth("/scripts/read").json(script).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response.json::<JsonScriptResponse>().await.unwrap())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub async fn list_scripts(&self) -> Result<Vec<JsonScriptInfo>, StatusCode> {
        match self
            .post_auth("/scripts/read/all")
            .json(&Empty {})
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let response = response.json::<JsonListResponse>().await.unwrap();
                    Ok(response.scripts)
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub async fn update_script(
        &self,
        script: &JsonScript,
    ) -> Result<JsonErrorResponse, StatusCode> {
        match self.post_auth("/scripts/update").json(script).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response.json::<JsonErrorResponse>().await.unwrap())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub async fn delete_script(&self, script: &JsonScriptName) -> Result<(), StatusCode> {
        match self.post_auth("/scripts/delete").json(script).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<Empty>().await.expect("empty JSON response");
                    Ok(())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }
}
