use std::{
    fs::create_dir_all,
    io,
    path::{Path, PathBuf},
};

use acme_lib::{create_p384_key, persist::FilePersist, Certificate, Directory, DirectoryUrl};
use jwt_simple::prelude::ES256KeyPair;
use once_cell::sync::OnceCell;
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use warp::{Filter, Rejection};

pub static JWT_KEY: OnceCell<ES256KeyPair> = OnceCell::new();
pub static TLS_CERT: OnceCell<Certificate> = OnceCell::new();

static JWT_KEY_FILE: &str = "jwt_key";

#[derive(Debug, Error)]
pub enum CertificateError {
    #[error("acme error: {0}")]
    AcmeError(#[from] acme_lib::Error),
    #[error("token save error: {0}")]
    ChallengeSave(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("could not interact with key: {0}")]
    KeyIoError(#[from] io::Error),
    #[error("failed to use JWT key: {0}")]
    KeyError(#[from] jwt_simple::Error),
}

#[tracing::instrument(name = "retrieve certificate")]
pub async fn retrieve_certificate(
    key_path: &Path,
    domain: &str,
    email: &str,
) -> Result<(), CertificateError> {
    if !load_certificate(key_path, domain)? {
        let challenge_server = build_acme_challenge_server();
        let challenge_handle =
            tokio::spawn(async move { challenge_server.run(([0, 0, 0, 0], 80)).await });

        request_certificate(key_path, domain, email).await?;

        challenge_handle.abort();
    }

    Ok(())
}

#[tracing::instrument(name = "retrieve jwt key", skip_all, fields(key_file = JWT_KEY_FILE))]
pub async fn retrieve_jwt_key(path: &Path) -> Result<(), JwtError> {
    let path = path.join(JWT_KEY_FILE);

    let key = if path.exists() {
        tracing::info!("loading JWT key from disk");
        let mut key_file = File::open(path).await?;
        let mut key = Vec::new();
        key_file.read_to_end(&mut key).await?;
        ES256KeyPair::from_bytes(key.as_slice())?
    } else {
        tracing::info!("generating new JWT key");
        let key = ES256KeyPair::generate();
        let mut key_file = File::create(path).await?;
        key_file.write_all(key.to_bytes().as_slice()).await?;
        key
    };

    if JWT_KEY.set(key).is_err() {
        panic!("unable to set JWT key, key already set");
    };

    Ok(())
}

fn build_acme_challenge_server(
) -> warp::Server<impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone> {
    let path = PathBuf::from("acme");
    create_dir_all(path.as_path()).unwrap();

    let routes = warp::path(".well-known")
        .and(warp::path("acme-challenge"))
        .and(warp::filters::fs::dir("acme"));

    warp::serve(routes)
}

#[tracing::instrument(name = "load certificate", skip_all)]
fn load_certificate(key_path: &Path, domain: &str) -> Result<bool, CertificateError> {
    let url = DirectoryUrl::LetsEncryptStaging;
    let persist = FilePersist::new(key_path);
    let directory = Directory::from_url(persist, url)?;

    let account = directory.account("sriler@gmail.com")?;
    if let Some(certificate) = account.certificate(domain)? {
        tracing::info!("loading TLS certificate from disk");
        TLS_CERT.set(certificate).unwrap();
        Ok(true)
    } else {
        tracing::info!("failed to locate TLS certificate",);
        Ok(false)
    }
}

#[tracing::instrument(name = "request certificate", skip_all)]
async fn request_certificate(
    key_path: &Path,
    domain: &str,
    email: &str,
) -> Result<(), CertificateError> {
    tracing::info!("requesting new TLS certificate");
    let url = DirectoryUrl::LetsEncrypt;
    let persist = FilePersist::new(key_path);
    let directory = Directory::from_url(persist, url)?;

    let account = directory.account(email)?;

    let mut new_order = account.new_order(domain, &[])?;

    let order_csr = loop {
        if let Some(order_csr) = new_order.confirm_validations() {
            break order_csr;
        }

        let auths = new_order.authorizations()?;
        let challenge = auths[0].http_challenge();

        save_token(challenge.http_token(), challenge.http_proof()).await?;

        challenge.validate(5000)?;
        new_order.refresh()?;
    };

    let key = create_p384_key();
    let order_certificate = order_csr.finalize_pkey(key, 5000)?;

    let certificate = order_certificate.download_and_save_cert()?;
    TLS_CERT.set(certificate).unwrap();

    tracing::info!("new certificate signed and saved");

    Ok(())
}

async fn save_token(token: &str, proof: String) -> Result<(), CertificateError> {
    let path = PathBuf::from("acme");
    let mut file = File::create(path.join(token)).await?;
    file.write_all(proof.as_bytes()).await?;
    Ok(())
}
