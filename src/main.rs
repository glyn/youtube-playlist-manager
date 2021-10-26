mod youtube_manager;

use clap::{App, Arg, SubCommand};
use google_youtube3::{Result, YouTube};
use hyper;
use hyper_rustls;
use std::str::FromStr;
use tokio;
use youtube_manager::playlist::Playlist;
use yup_oauth2::{read_application_secret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

fn main() -> Result<()> {
    let matches = App::new("stream-inspector")
        .arg(
            Arg::with_name("playlist_id")
                .help("A playlist id")
                .index(1) // Starts at 1
                .required(true),
        )
        .arg(
            Arg::with_name("secret")
                .help("Path to YouTube client secret file")
                .long_help("Path to YouTube client secret file. See https://github.com/glyn/stream-inspector for how to create this.")
                .takes_value(true)
                .long("secret")
                .required(true),
        )
        .arg(
            Arg::with_name("timezone")
                .help("A timezone for displaying dates and times, e.g. Europe/London")
                .takes_value(true)
                .long("timezone")
                .default_value("UTC"),
        )
        .arg(
            Arg::with_name("dry-run")
                .help("Goes through the motions without making any changes on YouTube")
                .takes_value(true)
                .long("dry-run")
                .default_value("true"), // for safety
        )
        .arg(
            Arg::with_name("debug")
                .help("Prints extra debugging information")
                .long("debug")
                .takes_value(false),
        )
        .subcommand(
     SubCommand::with_name("sort")
                .about("Sorts, and optionally prunes, the playlist")
                .arg(
                    Arg::with_name("prune")
                        .help("Removes extraneous entries from the playlist")
                        .long("prune")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("max playable")
                        .help("Maximum number of playable videos in the playlist. Others may be pruned.")
                        .long("max-playable")
                        .takes_value(true)
                        .default_value("6"),
                ),
        )
        .get_matches();

    let mut sort = false;
    let mut prune = false;
    let mut max_playable = 6;

    match matches.subcommand() {
        (_, Some(sub_matches)) => {
            sort = true;
            max_playable = sub_matches
                .value_of("max playable")
                .unwrap()
                .to_string()
                .parse::<usize>()
                .unwrap();
            prune = sub_matches.is_present("prune");
        }
        _ => {}
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async_main(
            matches.value_of("playlist_id").unwrap().to_owned(),
            matches.value_of("secret").unwrap().to_string(),
            matches.value_of("timezone").unwrap().to_string(),
            FromStr::from_str(matches.value_of("dry-run").unwrap()).unwrap_or(true),
            matches.is_present("debug"),
            sort,
            prune,
            max_playable,
        ))
}

async fn async_main(
    playlist: String,
    secret_path: String,
    timezone: String,
    dry_run: bool,
    debug: bool,
    sort: bool,
    prune: bool,
    max_catch_up: usize,
) -> Result<()> {
    let secret = read_application_secret(secret_path).await.unwrap();

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

    if !sort {
        eprintln!("Input playlist:");
    }
    play_list.print().await?;

    if sort {
        if prune {
            eprintln!("\nSorting and pruning...");
            play_list.prune(max_catch_up).await?;
        } else {
            eprintln!("\nSorting...");
            play_list.sort().await?;
        }
    }

    if !dry_run {
        if sort {
            eprintln!("Done.");
            eprintln!("\nOutput playlist:");
            play_list.print().await?;
        }
    } else {
        eprintln!(
            "\nThis was a dry run. To enable changes to the YouTube playlist, use --dry-run=false"
        );
    }

    Ok(())
}
