use std::env;

use remud_lib::run_remud;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Running ReMUD from {:?}", env::current_dir());

    let db = Some("./world.db");

    run_remud(db).await
}
