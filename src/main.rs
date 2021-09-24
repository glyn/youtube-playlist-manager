extern crate google_youtube3 as youtube3;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2 as oauth2;
use std::env;
use tokio;
use youtube3::YouTube;
use youtube3::{Error, Result};

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

    let result = hub
        .videos()
        .list(&vec!["contentDetails".into()])
        .chart("mostPopular")
        .doit()
        .await;

    match result {
        Err(e) => match e {
            // The Error enum provides details about what exactly happened.
            // You can also just use its `Debug`, `Display` or `Error` traits
            Error::HttpError(_)
            | Error::Io(_)
            | Error::MissingAPIKey
            | Error::MissingToken(_)
            | Error::Cancelled
            | Error::UploadSizeLimitExceeded(_, _)
            | Error::Failure(_)
            | Error::BadRequest(_)
            | Error::FieldClash(_)
            | Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(res) => println!("Success: {:?}", res),
    }
    Ok(())
}
