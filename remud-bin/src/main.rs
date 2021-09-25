use std::{env, fs::create_dir_all, io, path::PathBuf};

use anyhow::bail;
use clap::{App, Arg};
use remud_lib::run_remud;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let matches = App::new("ReMUD")
        .version("0.1")
        .author("Shaen & krixi - https://github.com/siler/remud")
        .about("A MUD in Rust.")
        .arg(
            Arg::new("telnet")
                .short('t')
                .long("telnet")
                .default_value("2004")
                .about("Sets the telnet port")
                .takes_value(true),
        )
        .arg(
            Arg::new("web")
                .short('w')
                .long("web")
                .default_value("2080")
                .about("Sets the web API port")
                .takes_value(true),
        )
        .arg(
            Arg::new("db")
                .short('d')
                .long("db")
                .default_value("./world.db")
                .about("Sets the database file path")
                .takes_value(true),
        )
        .arg(Arg::new("in-memory").long("in-memory").about(
            "Runs ReMUD with an in-memory SQLite database - all data will be lost when the \
             program is closed",
        ))
        .get_matches();

    let db = if matches.is_present("in-memory") {
        None
    } else {
        let path_str = matches.value_of("db").unwrap();

        let path = PathBuf::from(path_str);

        // Validate the database path, creating directories if necessary.
        if path.is_dir() {
            bail!("Parameter 'db' must be a filename, not a directory.");
        }

        if let Some(parent) = path.parent() {
            match parent.metadata() {
                Ok(_) => (),
                Err(e) => {
                    if e.kind() == io::ErrorKind::NotFound {
                        if let Err(e) = create_dir_all(parent) {
                            bail!("Failed to create directory path for database: {}", e);
                        }
                    } else {
                        bail!("Unable to access database parent directory: {}", e);
                    }
                }
            }
        } else {
            bail!(
                "Unable to determine parent directory of database: {:?}",
                path.as_os_str()
            );
        }

        Some(path_str)
    };

    let telnet = parse_port(matches.value_of("telnet").unwrap())?;
    let web = parse_port(matches.value_of("web").unwrap())?;

    let db_str = db.unwrap_or("in-memory");

    let telnet_addr = format!("0.0.0.0:{}", telnet);
    let web_addr = format!("0.0.0.0:{}", web);

    let cwd = env::current_dir();
    let dir = match &cwd {
        Ok(path) => path.to_str(),
        Err(e) => bail!("Cannot determine current working directory: {}", e),
    };

    match dir {
        Some(dir) => tracing::info!("Running ReMUD from {:?} with:", dir),
        None => tracing::info!("Running Remud with:"),
    }
    tracing::info!("  database: {}", db_str);
    tracing::info!("  telnet: {}", telnet_addr);
    tracing::info!("  web: {}", web_addr);

    run_remud(telnet_addr.as_str(), web_addr.as_str(), db, None).await?;

    Ok(())
}

fn parse_port(port: &str) -> anyhow::Result<u16> {
    let port = match port.parse::<u16>() {
        Ok(port) => port,
        Err(_) => bail!("Ports should be an integer between 1024 and 65,535 inclusive."),
    };

    Ok(port)
}
