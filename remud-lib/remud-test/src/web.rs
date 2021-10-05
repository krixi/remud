use std::borrow::Cow;

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

#[derive(Debug, Serialize)]
pub struct JsonScript {
    name: String,
    trigger: String,
    code: String,
}

#[derive(Debug, Serialize)]
pub struct JsonScriptName {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JsonListResponse {
    scripts: Vec<JsonScriptInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonScriptResponse {
    name: String,
    trigger: String,
    code: String,
    error: Option<JsonErrorInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonErrorResponse {
    error: Option<JsonErrorInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonScriptInfo {
    name: String,
    trigger: String,
    lines: i64,
    error: Option<JsonErrorInfo>,
}

#[derive(Debug, Deserialize)]
pub struct JsonErrorInfo {
    line: Option<i64>,
    position: Option<i64>,
    message: String,
}

#[derive(Clone)]
pub struct WebClient {
    port: u16,
    client: reqwest::blocking::Client,
}

impl WebClient {
    const URL: &'static str = "http://localhost";

    pub fn new(port: u16) -> Self {
        let client = reqwest::blocking::Client::new();
        WebClient { port, client }
    }

    pub fn login<'a, S1, S2>(
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
            .send()
        {
            Ok(response) => {
                if response.status().is_success() {
                    let response: JsonAuthResponse = response.json().unwrap();
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

    fn post(&self, path: &str) -> reqwest::blocking::RequestBuilder {
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
    fn post_auth(&self, path: &str) -> reqwest::blocking::RequestBuilder {
        self.client
            .post(path)
            .bearer_auth(self.access_token.as_str())
    }

    pub fn refresh_auth(&mut self) -> Result<(), StatusCode> {
        match self
            .client
            .post("/auth/refresh")
            .json(&JsonRefresh {
                refresh_token: self.refresh_token.clone(),
            })
            .send()
        {
            Ok(response) => {
                if response.status().is_success() {
                    let response = response.json::<JsonAuthResponse>().unwrap();
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
    pub fn logout(&self) -> Result<(), StatusCode> {
        match self.post_auth("/auth/logout").json(&Empty {}).send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<Empty>().unwrap();
                    Ok(())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub fn create_script(&self, script: &JsonScript) -> Result<Option<JsonErrorInfo>, StatusCode> {
        match self.post_auth("/scripts/create").json(script).send() {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response.json::<JsonErrorResponse>().unwrap().error)
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub fn read_script(&self, script: &JsonScriptName) -> Result<JsonScriptResponse, StatusCode> {
        match self.post_auth("/scripts/read").json(script).send() {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response.json::<JsonScriptResponse>().unwrap())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub fn list_scripts(&self) -> Result<Vec<JsonScriptInfo>, StatusCode> {
        match self.post_auth("/scripts/read/all").json(&Empty {}).send() {
            Ok(response) => {
                if response.status().is_success() {
                    let response = response.json::<JsonListResponse>().unwrap();
                    Ok(response.scripts)
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub fn update_script(&self, script: &JsonScriptName) -> Result<JsonScriptResponse, StatusCode> {
        match self.post_auth("/scripts/update").json(script).send() {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response.json::<JsonScriptResponse>().unwrap())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }

    pub fn delete_script(&self, script: &JsonScriptName) -> Result<(), StatusCode> {
        match self.post_auth("/scripts/delete").json(script).send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<Empty>().expect("empty JSON response");
                    Ok(())
                } else {
                    Err(response.status())
                }
            }
            Err(e) => Err(e.status().unwrap()),
        }
    }
}
