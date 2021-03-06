mod youtube_manager;

use clap::{App, Arg, SubCommand};
use env_logger;
use env_logger::Logger;
use google_youtube3::{Result, YouTube};
use hyper;
use hyper_rustls;
use log::debug;
use std::future::Future;
use std::pin::Pin;
use tokio;
use tokio::io::AsyncBufReadExt;
use webbrowser;
use youtube_manager::playlist::Playlist;
use yup_oauth2::{
    authenticator_delegate::InstalledFlowDelegate, read_application_secret,
    InstalledFlowAuthenticator, InstalledFlowReturnMethod,
};

const DELAY: u64 = 5;

fn main() -> Result<()> {
    let logger = Logger::from_default_env();
    async_log::Logger::wrap(logger, || 12)
        .start(log::LevelFilter::Trace)
        .unwrap();
    let matches = App::new("playlist-manager")
        .arg(
            Arg::with_name("playlist_id")
                .help("A playlist id")
                .index(1) // Starts at 1
                .required(true),
        )
        .arg(
            Arg::with_name("client")
                .help("Path to YouTube client id file")
                .long_help("Path to YouTube client id file. See https://github.com/glyn/youtube-playlist-manager for how to create this.")
                .takes_value(true)
                .long("client")
                .required(true),
        )
        .arg(
            Arg::with_name("timezone")
                .help("A timezone for displaying dates and times, e.g. Europe/London or UTC")
                .takes_value(true)
                .long("timezone")
                .default_value(""),
        )
        .arg(
            Arg::with_name("debug")
                .help("Prints extra debugging information")
                .long("debug")
                .takes_value(false),
        ).arg(
            Arg::with_name("pause")
                .help("pause for user input before terminating")
                .long("pause")
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
                )
                .arg(
                    Arg::with_name("update")
                        .help("Update YouTube")
                        .takes_value(false)
                        .long("update"),
                ),
        )
        .get_matches();

    let mut sort = false;
    let mut prune = false;
    let mut max_playable = 6;
    let mut dry_run = true;

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
            dry_run = !sub_matches.is_present("update");
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
            matches.value_of("client").unwrap().to_string(),
            matches.value_of("timezone").unwrap().to_string(),
            dry_run,
            matches.is_present("debug"),
            matches.is_present("pause"),
            sort,
            prune,
            max_playable,
        ))
}

async fn async_main(
    playlist: String,
    client_id_path: String,
    timezone: String,
    dry_run: bool,
    debug: bool,
    pause: bool,
    sort: bool,
    prune: bool,
    max_catch_up: usize,
) -> Result<()> {
    let client_id = read_application_secret(client_id_path).await.unwrap();

    // Create an authenticator that uses an InstalledFlow to authenticate. The
    // authentication tokens are persisted to a file. The
    // authenticator takes care of caching tokens to disk and refreshing tokens once
    // they've expired. Use a custom flow delegate to launch a browser automatically.
    debug!("building installed flow authenticator");
    let auth =
        InstalledFlowAuthenticator::builder(client_id, InstalledFlowReturnMethod::HTTPRedirect)
            .flow_delegate(Box::new(CustomInstalledFlowDelegate))
            .persist_tokens_to_disk("playlist-manager-tokencache.json")
            .build()
            .await
            .unwrap();
    debug!("installed flow authenticator built successfully");

    let hub = YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    );

    let play_list = youtube_manager::playlist::new(hub, &playlist, timezone, dry_run, debug);

    if sort {
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

        if !dry_run {
            eprintln!("Done.");
            eprintln!("\nOutput playlist:");
            play_list.print().await?;
        } else {
            eprintln!(
                "\nThis was only a dry run. To make changes to the YouTube playlist, repeat the command and add --update."
            );
        }
    }

    if pause {
        println!("\nCommand complete, press return to exit...");
        let mut user_input = String::new();
        tokio::io::BufReader::new(tokio::io::stdin())
            .read_line(&mut user_input)
            .await?;
    }

    Ok(())
}

#[derive(Copy, Clone)]
pub struct CustomInstalledFlowDelegate;
impl InstalledFlowDelegate for CustomInstalledFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = core::result::Result<String, String>> + Send + 'a>> {
        Box::pin(present_user_url(url, need_code))
    }
}

async fn present_user_url(url: &str, need_code: bool) -> core::result::Result<String, String> {
    if need_code {
        println!(
            "Launching a browser.\n\n(If a browser does not appear, please copy this link into a browser's address bar and press return: {})\n\nFollow the instructions displayed in the browser and enter the code displayed here: ",
            url
        );
        launch_browser(url).await?;
        let mut user_input = String::new();
        tokio::io::BufReader::new(tokio::io::stdin())
            .read_line(&mut user_input)
            .await
            .map_err(|e| format!("couldn't read code: {}", e))?;
        // remove trailing whitespace.
        user_input.truncate(user_input.trim_end().len());
        Ok(user_input)
    } else {
        println!(
            "Launching a browser.\n\n(If a browser does not appear, please copy this link into a browser's address bar and press return: {})\n\nFollow the instructions displayed in the browser.",
            url
        );
        launch_browser(url).await
    }
}

async fn launch_browser(url: &str) -> core::result::Result<String, String> {
    std::thread::sleep(std::time::Duration::from_secs(DELAY));
    if let Err(err) = webbrowser::open(url) {
        std::result::Result::Err(err.to_string())
    } else {
        Ok(String::new())
    }
}
