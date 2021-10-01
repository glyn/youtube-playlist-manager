# YouTube stream inspector

Work in progress!

## Authentication and authorisation

This uses an OAuth2 client ID to access the YouTube API. Read the [instructions](https://developers.google.com/identity/protocols/oauth2#installed) for how to create a client ID and download a file containing the client ID and its private key.

Specify the client secret file path using the `YOUTUBE_CLIENT_SECRET_FILE` environment variable.

## Command line interface

WIP, but currently you can run the code like this, where `PLz-8ZbAJhahjvkPtduhnB4TzhVcj5ZtfC` is a suitable playlist id:

YOUTUBE_CLIENT_SECRET_FILE=/path/to/client_secret_663812898511-gkjnnh493ar17niq2e7qv13pk9vgiqvv.apps.googleusercontent.com.json cargo run -- PLz-8ZbAJhahjvkPtduhnB4TzhVcj5ZtfC