mod youtube_manager;

use clap::{App, Arg};
use google_youtube3::{Result, YouTube};
use hyper;
use hyper_rustls;
use std::env;
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
        .get_matches();

    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async_main(
            matches.value_of("playlist_id").unwrap().to_owned(),
        ))
}

async fn async_main(playlist: String) -> Result<()> {
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

    let play_list = youtube_manager::playlist::new(hub, &playlist);

    println!("Input playlist:");
    print_videos(&play_list).await?;

    println!("\nPruning...");
    play_list.prune(6).await?;

    println!("Done.");

    println!("\nOutput playlist:");
    print_videos(&play_list).await?;

    Ok(())
}

async fn print_videos(play_list: &dyn youtube_manager::playlist::Playlist) -> Result<()> {
    for video in play_list.items().await? {
        println!(
            "{}: {} {:?} {:?} {}",
            video.video_id,
            video.title,
            video.scheduled_start_time,
            video.actual_start_time,
            if video.scheduled_start_time.is_none() {
                "** invalid"
            } else {
                ""
            }
        );
    }
    Ok(())
}
