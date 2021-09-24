use std::env;

use remud_lib::{run_remud, RemudError};

#[tokio::main]
async fn main() -> Result<(), RemudError> {
    tracing_subscriber::fmt::init();

    tracing::info!("Running ReMUD from {:?}", env::current_dir());

    let db = Some("./world.db");

    let telnet_addr = "0.0.0.0:2004";
    let web_addr = "0.0.0.0:2080";

    run_remud(telnet_addr, web_addr, db, None).await
}
