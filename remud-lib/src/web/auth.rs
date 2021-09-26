use std::{collections::HashSet, convert::TryFrom};

use async_trait::async_trait;
use itertools::Itertools;
use jwt_simple::prelude::{
    Claims, Duration, ECDSAP256KeyPairLike, ECDSAP256PublicKeyLike, VerificationOptions,
};
use serde::{Deserialize, Serialize};
use tide::{
    http::{headers::HeaderValue, mime},
    security::{CorsMiddleware, Origin},
    Body, Request, Response, Server, StatusCode,
};
use tide_http_auth::{Authentication, BearerAuthRequest, BearerAuthScheme, Storage};

use crate::{
    engine::db::AuthDb,
    web::{Context, Player},
    TOKEN_KEY,
};

pub fn auth_endpoint<DB>(context: Context<DB>) -> Server<Context<DB>>
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    let mut auth = tide::with_state(context);

    let cors = CorsMiddleware::new()
        .allow_methods("POST".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"));

    auth.with(cors);
    auth.with(Authentication::new(BearerAuthScheme::default()));
    auth.at("/login").post(login);
    auth.at("/refresh").post(refresh);
    auth.at("/logout").post(logout);

    auth
}

#[async_trait]
impl<DB> Storage<Player, BearerAuthRequest> for Context<DB>
where
    DB: AuthDb + Clone + Send + Sync + 'static,
{
    async fn get_user(&self, request: BearerAuthRequest) -> tide::Result<Option<Player>> {
        let mut issuers = HashSet::new();
        issuers.insert("remud".to_string());
        let mut audiences = HashSet::new();
        audiences.insert("remud".to_string());

        // Verify the token signature, issuer, and audience
        let claims = match TOKEN_KEY.public_key().verify_token::<TokenData>(
            request.token.as_str(),
            Some(VerificationOptions {
                allowed_issuers: Some(issuers),
                allowed_audiences: Some(audiences),
                ..Default::default()
            }),
        ) {
            Ok(claims) => claims,
            Err(e) => {
                tracing::warn!("failed to validate access bearer token: {}", e);
                return Ok(None);
            }
        };

        // Confirm this is an access token
        if !claims.custom.scopes.contains(&"access".to_string()) {
            return Ok(None);
        }

        let player = claims.subject.as_ref().unwrap().as_str();

        // Confirm that this access token hasn't been refreshed
        let access_issued = match self.db.access_issued_secs(player).await {
            Ok(issued) => issued,
            Err(e) => {
                tracing::error!("failed to retrieve access token issue time: {}", e);
                return Ok(None);
            }
        };
        if claims.issued_at.unwrap().as_secs() as i64 != access_issued {
            if let Err(e) = self.db.logout(player).await {
                tracing::error!("failed to log out player: {}", e);
            }
            return Ok(None);
        }

        let access = claims
            .custom
            .scopes
            .into_iter()
            .filter(|s| s != "access")
            .collect_vec();

        Ok(Some(Player {
            name: player.to_string(),
            access,
        }))
    }
}

#[derive(Debug, Deserialize)]
struct JsonTokenRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct JsonRefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct JsonTokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenData {
    scopes: Vec<String>,
}

async fn login<DB: AuthDb>(mut req: Request<Context<DB>>) -> tide::Result {
    let request = req.body_json::<JsonTokenRequest>().await?;
    let player = request.username.as_str();

    match req
        .state()
        .db
        .verify_player(player, request.password.as_str())
        .await
    {
        Ok(true) => (),
        Ok(false) => return Ok(Response::new(StatusCode::Unauthorized)),
        Err(e) => {
            tracing::error!("Failed to verify player during token request: {}", e);
            return Ok(Response::new(StatusCode::InternalServerError));
        }
    }

    let immortal = req.state().db.is_immortal(player).await?;

    let (access_token, access_issued_secs, refresh_token, refresh_issued_secs) =
        generate_tokens(player, immortal)?;

    req.state()
        .db
        .register_tokens(player, access_issued_secs, refresh_issued_secs)
        .await?;

    let response = JsonTokenResponse {
        access_token,
        refresh_token,
    };

    Ok(Response::builder(200)
        .body(Body::from_json(&response)?)
        .content_type(mime::JSON)
        .build())
}

async fn refresh<DB: AuthDb>(mut req: Request<Context<DB>>) -> tide::Result {
    let request = req.body_json::<JsonRefreshRequest>().await?;

    let mut issuers = HashSet::new();
    issuers.insert("remud".to_string());
    let mut audiences = HashSet::new();
    audiences.insert("remud".to_string());

    let claims = TOKEN_KEY.public_key().verify_token::<TokenData>(
        request.refresh_token.as_str(),
        Some(VerificationOptions {
            allowed_issuers: Some(issuers),
            allowed_audiences: Some(audiences),
            ..Default::default()
        }),
    )?;

    // Confirm this is a refresh token
    if !claims.custom.scopes.contains(&"refresh".to_string()) {
        return Ok(Response::new(StatusCode::Unauthorized));
    }

    let player = claims.subject.as_ref().unwrap().as_str();

    // Confirm that this refresh token hasn't already been used. If it has, log out the player.
    let refresh_issued = req.state().db.refresh_issued_secs(player).await?;
    if claims.issued_at.unwrap().as_secs() as i64 != refresh_issued {
        req.state().db.logout(player).await?;
        return Ok(Response::new(StatusCode::Unauthorized));
    }

    // Generate a new set of access/refresh tokens.
    let immortal = req.state().db.is_immortal(player).await?;

    let (access_token, access_issued_secs, refresh_token, refresh_issued_secs) =
        generate_tokens(player, immortal)?;

    req.state()
        .db
        .register_tokens(player, access_issued_secs, refresh_issued_secs)
        .await?;

    let response = JsonTokenResponse {
        access_token,
        refresh_token,
    };

    Ok(Response::builder(200)
        .body(Body::from_json(&response)?)
        .content_type(mime::JSON)
        .build())
}

async fn logout<DB: AuthDb>(req: Request<Context<DB>>) -> tide::Result {
    if let Some(player) = req.ext::<Player>() {
        req.state().db.logout(player.name.as_str()).await?;
        Ok(Response::new(StatusCode::Ok))
    } else {
        let mut response = Response::new(StatusCode::Unauthorized);
        response.append_header("WWW-Authenticate", "Bearer");
        Ok(response)
    }
}

pub fn generate_tokens(player: &str, immortal: bool) -> anyhow::Result<(String, i64, String, i64)> {
    let mut scopes = vec!["access".to_string()];

    if immortal {
        scopes.push("scripts".to_string());
    }

    let access_data = TokenData { scopes };
    let access_claims = Claims::with_custom_claims(access_data, Duration::from_hours(1))
        .with_issuer("remud")
        .with_audience("remud")
        .with_subject(player);
    let access_issued = access_claims.issued_at.unwrap();
    let access_token = TOKEN_KEY.sign(access_claims)?;

    let refresh_data = TokenData {
        scopes: vec!["refresh".to_string()],
    };
    let refresh_claims = Claims::with_custom_claims(refresh_data, Duration::from_days(365))
        .with_issuer("remud")
        .with_audience("remud")
        .with_subject(player);
    let refresh_issued = refresh_claims.issued_at.unwrap();
    let refresh_token = TOKEN_KEY.sign(refresh_claims)?;

    let access_issued_secs = i64::try_from(access_issued.as_secs())?;
    let refresh_issued_secs = i64::try_from(refresh_issued.as_secs())?;

    Ok((
        access_token,
        access_issued_secs,
        refresh_token,
        refresh_issued_secs,
    ))
}
