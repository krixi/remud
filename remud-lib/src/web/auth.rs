use std::{collections::HashSet, convert::TryFrom};

use jwt_simple::prelude::{
    Claims, Duration, ECDSAP256KeyPairLike, ECDSAP256PublicKeyLike, VerificationOptions,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use warp::{reject, Filter, Rejection};

use crate::{
    engine::db::AuthDb,
    web::{security::JWT_KEY, with_db, InternalError, Player},
};

pub const SCOPE_SCRIPTS: &str = "scripts";

const TOKEN_ISSUER: &str = "remud";
const TOKEN_AUDIENCE: &str = "remud";
const SCOPE_ACCESS: &str = "access";
const SCOPE_REFRESH: &str = "refresh";

const TOKEN_ISSUERS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut issuers = HashSet::new();
    issuers.insert(TOKEN_ISSUER.to_string());
    issuers
});
const TOKEN_AUDIENCES: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut audiences = HashSet::new();
    audiences.insert(TOKEN_AUDIENCE.to_string());
    audiences
});

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

#[tracing::instrument(name = "verify access", skip(db))]
async fn handle_verify_access<DB: AuthDb>(
    auth_header: String,
    db: DB,
    scopes: Vec<String>,
) -> Result<Player, Rejection> {
    let token = match auth_header.split_whitespace().nth(1) {
        Some(token) => token,
        None => {
            tracing::warn!("received invalid Authorization header");
            return Err(reject::custom(AuthError::InvalidAuthHeader));
        }
    };

    // Verify the token signature, issuer, and audience
    let claims = match JWT_KEY
        .get()
        .unwrap()
        .public_key()
        .verify_token::<TokenData>(
            token,
            Some(VerificationOptions {
                allowed_issuers: Some(TOKEN_ISSUERS.clone()),
                allowed_audiences: Some(TOKEN_AUDIENCES.clone()),
                ..Default::default()
            }),
        ) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::warn!("token could not be verified: {}", e);
            return Err(reject::custom(AuthError::VerificationError));
        }
    };

    // Confirm this is an access token
    if !claims.custom.scopes.contains(&SCOPE_ACCESS.to_string()) {
        tracing::warn!("token missing access scope");
        return Err(reject::custom(AuthError::InadequateAccess));
    }

    for scope in &scopes {
        if !claims.custom.scopes.contains(scope) {
            tracing::warn!("missing required scope: {}", scope);
            return Err(reject::custom(AuthError::InadequateAccess));
        }
    }

    let player = claims.subject.as_ref().unwrap().as_str();

    // Confirm that this access token hasn't been refreshed
    let access_issued = match db.access_issued_secs(player).await {
        Ok(issued) => match issued {
            Some(issued) => issued,
            None => {
                tracing::warn!("player has no active tokens");
                return Err(reject::custom(AuthError::InvalidToken));
            }
        },
        Err(e) => {
            tracing::error!("failed to retrieve access token issue time: {}", e);
            return Err(reject::custom(InternalError {}));
        }
    };
    if claims.issued_at.unwrap().as_secs() as i64 != access_issued {
        tracing::warn!("client provided an expired token");
        if let Err(e) = db.logout(player).await {
            tracing::error!("failed to log out player: {}", e);
        }
        return Err(reject::custom(AuthError::InvalidToken));
    }

    Ok(Player {
        name: player.to_string(),
    })
}

#[tracing::instrument(name = "login", skip(db))]
async fn handle_login<DB: AuthDb>(
    request: JsonTokenRequest,
    db: DB,
) -> Result<impl warp::Reply, Rejection> {
    let player = request.username.as_str();

    match db.verify_player(player, request.password.as_str()).await {
        Ok(true) => (),
        Ok(false) => return Err(reject::custom(AuthError::AuthenticationError)),
        Err(e) => {
            tracing::error!("failed to verify player during token request: {}", e);
            return Err(reject::custom(InternalError {}));
        }
    }

    let immortal = match db.is_immortal(player).await {
        Ok(immortal) => immortal,
        Err(err) => {
            tracing::error!("failed to retrieve immortal status: {}", err);
            return Err(reject::custom(InternalError {}));
        }
    };

    let (access_token, access_issued_secs, refresh_token, refresh_issued_secs) =
        match generate_tokens(player, immortal) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("failed to generate tokens: {}", err);
                return Err(reject::custom(InternalError {}));
            }
        };

    if let Err(err) = db
        .register_tokens(player, access_issued_secs, refresh_issued_secs)
        .await
    {
        tracing::error!("failed to register web tokens: {}", err);
        return Err(reject::custom(InternalError {}));
    };

    let response = JsonTokenResponse {
        access_token,
        refresh_token,
    };

    Ok(warp::reply::json(&response))
}

#[tracing::instrument(name = "refresh", skip(db))]
async fn handle_refresh<DB: AuthDb>(
    request: JsonRefreshRequest,
    db: DB,
) -> Result<impl warp::Reply, Rejection> {
    let claims = match JWT_KEY
        .get()
        .unwrap()
        .public_key()
        .verify_token::<TokenData>(
            request.refresh_token.as_str(),
            Some(VerificationOptions {
                allowed_issuers: Some(TOKEN_ISSUERS.clone()),
                allowed_audiences: Some(TOKEN_AUDIENCES.clone()),
                ..Default::default()
            }),
        ) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::warn!("token could not be verified: {}", e);
            return Err(reject::custom(AuthError::VerificationError));
        }
    };

    // Confirm this is a refresh token
    if !claims.custom.scopes.contains(&SCOPE_REFRESH.to_string()) {
        tracing::warn!("token is not a refresh token");
        return Err(reject::custom(AuthError::InadequateAccess));
    }

    let player = claims.subject.as_ref().unwrap().as_str();

    // Confirm that this refresh token hasn't already been used. If it has, log out the player.
    let refresh_issued = match db.refresh_issued_secs(player).await {
        Ok(issued) => match issued {
            Some(issued) => issued,
            None => {
                tracing::warn!("player has no valid tokens");
                return Err(reject::custom(AuthError::InvalidToken));
            }
        },
        Err(err) => {
            tracing::error!("failed to retrieve refresh token issue time: {}", err);
            return Err(reject::custom(InternalError {}));
        }
    };

    if claims.issued_at.unwrap().as_secs() as i64 != refresh_issued {
        tracing::warn!("client attempting to use expired token");
        if let Err(err) = db.logout(player).await {
            tracing::error!("failed to log out player using expired token: {}", err);
            return Err(reject::custom(InternalError {}));
        };
        return Err(reject::custom(AuthError::InvalidToken));
    }

    // Generate a new set of access/refresh tokens.
    let immortal = match db.is_immortal(player).await {
        Ok(immortal) => immortal,
        Err(err) => {
            tracing::error!("failed to retrieve player immortal status: {}", err);
            return Err(reject::custom(InternalError {}));
        }
    };

    let (access_token, access_issued_secs, refresh_token, refresh_issued_secs) =
        match generate_tokens(player, immortal) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("failed to generate tokens: {}", err);
                return Err(reject::custom(InternalError {}));
            }
        };

    if let Err(err) = db
        .register_tokens(player, access_issued_secs, refresh_issued_secs)
        .await
    {
        tracing::error!("failed to register tokens: {}", err);
        return Err(reject::custom(InternalError {}));
    };

    let response = JsonTokenResponse {
        access_token,
        refresh_token,
    };

    Ok(warp::reply::json(&response))
}

#[tracing::instrument(name = "logout", skip(db))]
async fn handle_logout<DB: AuthDb>(db: DB, player: Player) -> Result<impl warp::Reply, Rejection> {
    if let Err(err) = db.logout(player.name.as_str()).await {
        tracing::error!("failed to log out player: {}", err);
        return Err(reject::custom(InternalError {}));
    };
    Ok(warp::reply())
}

fn generate_tokens(player: &str, immortal: bool) -> anyhow::Result<(String, i64, String, i64)> {
    let mut scopes = vec![SCOPE_ACCESS.to_string()];

    if immortal {
        scopes.push(SCOPE_SCRIPTS.to_string());
    }

    let access_data = TokenData { scopes };
    let access_claims = Claims::with_custom_claims(access_data, Duration::from_hours(1))
        .with_issuer(TOKEN_ISSUER)
        .with_audience(TOKEN_AUDIENCE)
        .with_subject(player);
    let access_issued = access_claims.issued_at.unwrap();
    let access_token = JWT_KEY.get().unwrap().sign(access_claims)?;

    let refresh_data = TokenData {
        scopes: vec![SCOPE_REFRESH.to_string()],
    };
    let refresh_claims = Claims::with_custom_claims(refresh_data, Duration::from_days(365))
        .with_issuer(TOKEN_ISSUER)
        .with_audience(TOKEN_AUDIENCE)
        .with_subject(player);
    let refresh_issued = refresh_claims.issued_at.unwrap();
    let refresh_token = JWT_KEY.get().unwrap().sign(refresh_claims)?;

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
