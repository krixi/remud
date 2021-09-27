use std::{collections::HashSet, convert::TryFrom};

use jwt_simple::prelude::{
    Claims, Duration, ECDSAP256KeyPairLike, ECDSAP256PublicKeyLike, VerificationOptions,
};
use serde::{Deserialize, Serialize};
use warp::{reject, Filter, Rejection};

use crate::{
    engine::db::AuthDb,
    web::{with_db, InternalError, Player},
    TOKEN_KEY,
};

pub fn auth_filters<DB>(
    db: DB,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("auth")
        .and(warp::post())
        .and(login(db.clone()).or(refresh(db.clone())).or(logout(db)))
}

#[derive(Debug, Deserialize)]
pub struct JsonTokenRequest {
    username: String,
    password: String,
}

fn json_login() -> impl Filter<Extract = (JsonTokenRequest,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

#[derive(Debug, Deserialize)]
pub struct JsonRefreshRequest {
    refresh_token: String,
}

fn json_refresh() -> impl Filter<Extract = (JsonRefreshRequest,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

#[derive(Debug, Serialize)]
struct JsonTokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenData {
    pub scopes: Vec<String>,
}

#[derive(Debug)]
pub enum AuthError {
    AuthenticationError,
    InvalidAuthHeader,
    InvalidToken,
    VerificationError,
    InadequateAccess,
}

impl reject::Reject for AuthError {}

pub fn verify_access<DB>(
    db: DB,
    scopes: Vec<String>,
) -> impl Filter<Extract = (Player,), Error = Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::header::<String>("Authorization")
        .and(with_db(db))
        .and(with_scopes(scopes))
        .and_then(handle_verify_access)
}

pub fn login<DB>(db: DB) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("login")
        .and(json_login())
        .and(with_db(db))
        .and_then(handle_login)
}

pub fn refresh<DB>(db: DB) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("refresh")
        .and(json_refresh())
        .and(with_db(db))
        .and_then(handle_refresh)
}

pub fn logout<DB>(db: DB) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone
where
    DB: AuthDb + Send + Sync + Clone + 'static,
{
    warp::path("logout")
        .and(with_db(db.clone()))
        .and(verify_access(db, vec![]))
        .and_then(handle_logout)
}

async fn handle_verify_access<DB: AuthDb>(
    auth_header: String,
    db: DB,
    scopes: Vec<String>,
) -> Result<Player, Rejection> {
    let token = match auth_header.split_whitespace().nth(1) {
        Some(token) => token,
        None => return Err(reject::custom(AuthError::InvalidAuthHeader)),
    };

    let mut issuers = HashSet::new();
    issuers.insert("remud".to_string());
    let mut audiences = HashSet::new();
    audiences.insert("remud".to_string());

    // Verify the token signature, issuer, and audience
    let claims = match TOKEN_KEY.public_key().verify_token::<TokenData>(
        token,
        Some(VerificationOptions {
            allowed_issuers: Some(issuers),
            allowed_audiences: Some(audiences),
            ..Default::default()
        }),
    ) {
        Ok(claims) => claims,
        Err(_) => {
            return Err(reject::custom(AuthError::VerificationError));
        }
    };

    // Confirm this is an access token
    if !claims.custom.scopes.contains(&"access".to_string()) {
        return Err(reject::custom(AuthError::InadequateAccess));
    }

    for scope in &scopes {
        if !claims.custom.scopes.contains(scope) {
            return Err(reject::custom(AuthError::InadequateAccess));
        }
    }

    let player = claims.subject.as_ref().unwrap().as_str();

    // Confirm that this access token hasn't been refreshed
    let access_issued = match db.access_issued_secs(player).await {
        Ok(issued) => issued,
        Err(e) => {
            tracing::error!("failed to retrieve access token issue time: {}", e);
            return Err(reject::custom(InternalError {}));
        }
    };
    if claims.issued_at.unwrap().as_secs() as i64 != access_issued {
        if let Err(e) = db.logout(player).await {
            tracing::error!("failed to log out player: {}", e);
        }
        return Err(reject::custom(AuthError::InvalidToken));
    }

    Ok(Player {
        name: player.to_string(),
    })
}

async fn handle_login<DB: AuthDb>(
    request: JsonTokenRequest,
    db: DB,
) -> Result<impl warp::Reply, Rejection> {
    let player = request.username.as_str();

    match db.verify_player(player, request.password.as_str()).await {
        Ok(true) => (),
        Ok(false) => return Err(reject::custom(AuthError::AuthenticationError)),
        Err(e) => {
            tracing::error!("Failed to verify player during token request: {}", e);
            return Err(reject::custom(InternalError {}));
        }
    }

    let immortal = match db.is_immortal(player).await {
        Ok(immortal) => immortal,
        Err(err) => {
            tracing::error!("Failed to retrieve immortal status: {}", err);
            return Err(reject::custom(InternalError {}));
        }
    };

    let (access_token, access_issued_secs, refresh_token, refresh_issued_secs) =
        match generate_tokens(player, immortal) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("Failed to generate tokens: {}", err);
                return Err(reject::custom(InternalError {}));
            }
        };

    if let Err(err) = db
        .register_tokens(player, access_issued_secs, refresh_issued_secs)
        .await
    {
        tracing::error!("Failed to register web tokens: {}", err);
        return Err(reject::custom(InternalError {}));
    };

    let response = JsonTokenResponse {
        access_token,
        refresh_token,
    };

    Ok(warp::reply::json(&response))
}

async fn handle_refresh<DB: AuthDb>(
    request: JsonRefreshRequest,
    db: DB,
) -> Result<impl warp::Reply, Rejection> {
    let mut issuers = HashSet::new();
    issuers.insert("remud".to_string());
    let mut audiences = HashSet::new();
    audiences.insert("remud".to_string());

    let claims = match TOKEN_KEY.public_key().verify_token::<TokenData>(
        request.refresh_token.as_str(),
        Some(VerificationOptions {
            allowed_issuers: Some(issuers),
            allowed_audiences: Some(audiences),
            ..Default::default()
        }),
    ) {
        Ok(claims) => claims,
        Err(_) => return Err(reject::custom(AuthError::VerificationError)),
    };

    // Confirm this is a refresh token
    if !claims.custom.scopes.contains(&"refresh".to_string()) {
        return Err(reject::custom(AuthError::InadequateAccess));
    }

    let player = claims.subject.as_ref().unwrap().as_str();

    // Confirm that this refresh token hasn't already been used. If it has, log out the player.
    let refresh_issued = match db.refresh_issued_secs(player).await {
        Ok(issued) => issued,
        Err(err) => {
            tracing::error!("Failed to retrieve refresh token issue time: {}", err);
            return Err(reject::custom(InternalError {}));
        }
    };

    if claims.issued_at.unwrap().as_secs() as i64 != refresh_issued {
        if db.logout(player).await.is_err() {
            return Err(reject::custom(InternalError {}));
        };
        return Err(reject::custom(AuthError::InvalidToken));
    }

    // Generate a new set of access/refresh tokens.
    let immortal = match db.is_immortal(player).await {
        Ok(immortal) => immortal,
        Err(err) => {
            tracing::error!("Failed to retrieve player immortal status: {}", err);
            return Err(reject::custom(InternalError {}));
        }
    };

    let (access_token, access_issued_secs, refresh_token, refresh_issued_secs) =
        match generate_tokens(player, immortal) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("Failed to generate tokens: {}", err);
                return Err(reject::custom(InternalError {}));
            }
        };

    if let Err(err) = db
        .register_tokens(player, access_issued_secs, refresh_issued_secs)
        .await
    {
        tracing::error!("Failed to register tokens: {}", err);
        return Err(reject::custom(InternalError {}));
    };

    let response = JsonTokenResponse {
        access_token,
        refresh_token,
    };

    Ok(warp::reply::json(&response))
}

async fn handle_logout<DB: AuthDb>(db: DB, player: Player) -> Result<impl warp::Reply, Rejection> {
    if let Err(err) = db.logout(player.name.as_str()).await {
        tracing::error!("Failed to log out player: {}", err);
        return Err(reject::custom(InternalError {}));
    };
    Ok(warp::reply())
}

fn generate_tokens(player: &str, immortal: bool) -> anyhow::Result<(String, i64, String, i64)> {
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

fn with_scopes(
    scopes: Vec<String>,
) -> impl Filter<Extract = (Vec<String>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || scopes.clone())
}
