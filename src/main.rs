mod youtube_manager;

use clap::{App, Arg};
use google_youtube3::{Result, YouTube};
use hyper;
use hyper_rustls;
use std::env;
use std::str::FromStr;
use tokio;
use youtube_manager::playlist::Playlist;
use yup_oauth2::{read_application_secret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

fn main() -> Result<()> {
    let matches = App::new("plough")
        .arg(
            Arg::with_name("playlist_id")
                .help("the playlist id")
                .index(1) // Starts at 1
                .required(true),
        )
        .arg(
            Arg::with_name("timezone")
                .help("the timezone for displaying dates and times, e.g. Europe/London")
                .takes_value(true)
                .long("timezone")
                .default_value("UTC"),
        )
        .arg(
            Arg::with_name("dry-run")
                .help("go through the motions without making any changes on YouTube")
                .takes_value(true)
                .long("dry-run")
                .default_value("true"), // for safety
        )
        .arg(
            Arg::with_name("debug")
                .help("print extra debugging information")
                .long("debug")
                .takes_value(false),
        )
        .get_matches();

    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async_main(
            matches.value_of("playlist_id").unwrap().to_owned(),
            matches.value_of("timezone").unwrap().to_string(),
            FromStr::from_str(matches.value_of("dry-run").unwrap()).unwrap_or(true),
            matches.is_present("debug"),
        ))
}

async fn async_main(playlist: String, timezone: String, dry_run: bool, debug: bool) -> Result<()> {
    let key = "YOUTUBE_CLIENT_SECRET_FILE";
    let client_secret_file;
    match env::var(key) {
        Ok(val) => client_secret_file = val,
        Err(e) => {
            panic!("Environment variable {} must be set to the file path of a Google API JSON service account file: {}", key, e);
        }
    }

    let secret = read_application_secret(client_secret_file).await.unwrap();

    // Create an authenticator that uses an InstalledFlow to authenticate. The
    // authentication tokens are persisted to a file. The
    // authenticator takes care of caching tokens to disk and refreshing tokens once
    // they've expired.
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("api_inspector_tokencache.json")
        .build()
        .await
        .unwrap();

    let hub = YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    );

    let play_list = youtube_manager::playlist::new(hub, &playlist, timezone, dry_run, debug);

    eprintln!("Input playlist:");
    play_list.print().await?;

    eprintln!("\nPruning...");
    play_list.prune(6).await?;

    if !dry_run {
        eprintln!("Done.");
        eprintln!("\nOutput playlist:");
        play_list.print().await?;
    } else {
        eprintln!(
            "\nThis was a dry run. To enable changes to the YouTube playlist, use --dry-run=false"
        );
    }

    Ok(())
}
