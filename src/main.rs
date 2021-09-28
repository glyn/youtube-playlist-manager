extern crate google_youtube3 as youtube3;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2 as oauth2;

mod youtube_manager;

use std::env;
use tokio;
use youtube3::{Result, YouTube};

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
    let key = "YOUTUBE_SERVICE_ACCOUNT_FILE";
    let service_account_file;
    match env::var(key) {
        Ok(val) => service_account_file = val,
        Err(e) => {
            panic!("Environment variable {} must be set to the file path of a Google API JSON service account file: {}", key, e);
        }
    }

    let service_account_key = oauth2::read_service_account_key(service_account_file)
        .await
        .unwrap();

    let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .expect("Failed to create authenticator");

    let hub = YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        authenticator,
    );

    let result = youtube_manager::playlist::playlist_items(&hub, PLAYLIST, &None).await;

    match result {
        Err(e) => println!("{}", e),
        Ok((_, mut res)) => {
            while res.next_page_token.is_some() {
                match &res.items {
                    Some(items) => {
                        for item in items {
                            let video = hub
                                .videos()
                                .list(&vec!["liveStreamingDetails".into()])
                                .add_id(
                                    item.content_details
                                        .as_ref()
                                        .unwrap()
                                        .video_id
                                        .as_ref()
                                        .unwrap(),
                                )
                                .doit()
                                .await;
                            match video {
                                Err(e) => println!("{}", e),
                                Ok((_, v)) => {
                                    let items = v.items.unwrap();
                                    {
                                        let live_streaming_details =
                                            items.get(0).unwrap().live_streaming_details.as_ref();
                                        if live_streaming_details.is_some()
                                            && live_streaming_details
                                                .unwrap()
                                                .actual_start_time
                                                .is_some()
                                        {
                                            let actual_start_time = live_streaming_details
                                                .unwrap()
                                                .actual_start_time
                                                .as_ref()
                                                .unwrap();

                                            println!(
                                                "{}:{}",
                                                item.snippet
                                                    .as_ref()
                                                    .unwrap()
                                                    .title
                                                    .as_ref()
                                                    .unwrap(),
                                                actual_start_time
                                            )
                                        } else {
                                            println!(
                                                "{}:future",
                                                item.snippet
                                                    .as_ref()
                                                    .unwrap()
                                                    .title
                                                    .as_ref()
                                                    .unwrap()
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None => (),
                }

                let result =
                    youtube_manager::playlist::playlist_items(&hub, PLAYLIST, &res.next_page_token)
                        .await;

                match result {
                    Err(e) => println!("{}", e),
                    Ok((_, next_res)) => res = next_res,
                }
            }
        }
    }
    Ok(())
}
