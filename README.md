# YouTube stream inspector

Work in progress!

## Authentication and authorisation

This uses a service account to access the YouTube API. Read the [instructions](https://developers.google.com/identity/protocols/oauth2/service-account) for how to create a service account and download a file containing the service account and its private key.

Specify the service account file path using the `YOUTUBE_SERVICE_ACCOUNT_FILE` environment variable.