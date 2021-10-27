# YouTube stream inspector

Work in progress!

## Authentication and authorisation

This uses an OAuth2 client ID to access the YouTube API. Read the [instructions](https://developers.google.com/identity/protocols/oauth2#installed) for how to create a client ID and download a file containing the client ID and its private key (note: the application type is a desktop app). Specify the path to this client id file using the `--client` switch.

## Command line interface

WIP, but you can run the code like this:

cargo run -- &lt;playlist-id&gt; --client=/path/to/client_id.json

Use `--help` for more information.