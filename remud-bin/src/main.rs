use std::{
    env,
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use anyhow::bail;
use clap::{App, Arg, ArgMatches};
use remud_lib::{run_remud, TlsOptions, WebOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let matches = App::new("ReMUD")
        .version("0.1")
        .author("Shaen & krixi - https://github.com/siler/remud")
        .about("A MUD in Rust.")
        .arg(
            Arg::new("cors")
                .short('c')
                .long("cors")
                .about("Specify which domains should be allowed origins via CORS.")
                .takes_value(true),
        )
        .arg(
            Arg::new("db")
                .short('d')
                .long("db")
                .default_value("./world.db")
                .about("Sets the database file path.")
                .takes_value(true),
        )
        .arg(
            Arg::new("email")
                .short('e')
                .long("email")
                .about(
                    "Specify a contact email for Let's Encrypt's automated TLS certificate \
                     process. Must be set when --tls is set.",
                )
                .takes_value(true),
        )
        .arg(Arg::new("in-memory").long("in-memory").about(
            "Runs ReMUD with an in-memory SQLite database - all data will be lost when the \
             program is closed",
        ))
        .arg(
            Arg::new("keys")
                .short('k')
                .long("keys")
                .default_value("./keys")
                .about("Sets the key storage path.")
                .takes_value(true),
        )
        .arg(
            Arg::new("telnet")
                .short('t')
                .long("telnet")
                .default_value("2004")
                .about("Sets the telnet port.")
                .takes_value(true),
        )
        .arg(
            Arg::new("tls")
                .short('s')
                .long("tls")
                .about("Enables TLS for the specified domain. Must be set when --email is set.")
                .takes_value(true),
        )
        .arg(
            Arg::new("web")
                .short('w')
                .long("web")
                .default_value("2080")
                .about("Sets the web API port.")
                .takes_value(true),
        )
        .get_matches();

    let db = parse_db(&matches)?;
    let keys = parse_keys(&matches)?;

    let telnet = parse_port(matches.value_of("telnet").unwrap())?;
    let web = parse_port(matches.value_of("web").unwrap())?;

    let cors: Vec<&str> = matches
        .value_of("cors")
        .map(|domains| domains.split(',').collect())
        .unwrap_or_else(Vec::new);

    let tls = parse_tls(&matches)?;
    let web = WebOptions::new(web, keys, cors, tls);

    let cwd = env::current_dir();
    let dir = match &cwd {
        Ok(path) => path.to_str(),
        Err(e) => bail!("cannot determine current working directory: {}", e),
    };

    match dir {
        Some(dir) => tracing::info!("running ReMUD from {:?} with:", dir),
        None => tracing::info!("running Remud with:"),
    }
    tracing::info!("  database: {}", db.unwrap_or("in-memory"));
    tracing::info!("  telnet: {}", format!("0.0.0.0:{}", telnet));
    tracing::info!("  web: {}", web.uri());
    tracing::info!("  cors: {:?}", web.cors());

    run_remud(db, telnet, web, None).await?;

    Ok(())
}

fn parse_db(matches: &ArgMatches) -> anyhow::Result<Option<&str>> {
    if matches.is_present("in-memory") {
        Ok(None)
    } else {
        let path_str = matches.value_of("db").unwrap();

        let path = PathBuf::from(path_str);

        // Validate the database path, creating directories if necessary.
        if path.is_dir() {
            bail!("parameter 'db' must be a filename, not a directory.");
        }

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = create_dir_all(parent) {
                    bail!("failed to create directory path for database path: {}", e);
                }
            }
        } else {
            bail!(
                "unable to determine parent directory of database path: {:?}",
                path.as_os_str()
            );
        }

        Ok(Some(path_str))
    }
}

fn parse_keys(matches: &ArgMatches) -> anyhow::Result<&Path> {
    let path_str = matches.value_of("keys").unwrap();

    let path = Path::new(path_str);

    // Validate the database path, creating directories if necessary.
    if path.exists() && !path.is_dir() {
        bail!("parameter 'key' must be a directory, not a file.");
    }

    if !path.exists() {
        if let Err(e) = create_dir_all(path) {
            bail!("failed to create directory path for key storage: {}", e);
        }
    }

    Ok(path)
}

fn parse_port(port: &str) -> anyhow::Result<u16> {
    let port = match port.parse::<u16>() {
        Ok(port) => port,
        Err(_) => bail!("ports should be an integer between 1024 and 65,535 inclusive."),
    };

    Ok(port)
}

fn parse_tls(matches: &ArgMatches) -> anyhow::Result<Option<TlsOptions>> {
    match (matches.value_of("tls"), matches.value_of("email")) {
        (Some(domain), Some(email)) => Ok(Some(TlsOptions::new(domain, email))),
        (None, None) => Ok(None),
        _ => {
            bail!("--tls and --email must be set together");
        }
    }
}
