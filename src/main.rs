mod youtube_manager;

use google_youtube3::{Result, YouTube};
use hyper;
use hyper_rustls;
use std::env;
use tokio;
use youtube_manager::playlist::Playlist;
use yup_oauth2::{read_application_secret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

const PLAYLIST: &str = "PLz-8ZbAJhahjvkPtduhnB4TzhVcj5ZtfC"; // "Christ Church Winchester | Church Online Catch Up"

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    let key = "YOUTUBE_CLIENT_SECRET_FILE";
    let client_secret_file;
    match env::var(key) {
        Ok(val) => client_secret_file = val,
        Err(e) => {
            panic!("Environment variable {} must be set to the file path of a Google API JSON service account file: {}", key, e);
        }
    }

    // let service_account_key = read_service_account_key(service_account_file)
    //     .await
    //     .unwrap();

    // let authenticator = ServiceAccountAuthenticator::builder(service_account_key)
    //     .build()
    //     .await
    //     .expect("Failed to create authenticator");

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

    let play_list = youtube_manager::playlist::new(hub, PLAYLIST);
    let videos = play_list.items().await?;

    for video in videos {
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

    println!("Pruning...");
    play_list.prune().await?;

    Ok(())
}
