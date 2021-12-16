# YouTube stream inspector

Work in progress!

## Authentication and authorisation

This uses an OAuth2 client ID to access the YouTube API. Read the [instructions](https://developers.google.com/identity/protocols/oauth2#installed) for how to create a client ID and download a file containing the client ID and its private key (follow the steps below). Specify the path to this client id file using the `--client` switch.

Steps:
1. Open the [Google API Console](https://console.developers.google.com/) and ensure `Dashboard` is selected in the navigation list on the left.
2. Use the project drop-down at the top to create a new project called, for example, `playlist-manager`.
3. Click `+ ENABLE APIS AND SERVICES`, search for `YouTube`, select `YouTube Data API v3`, and click `ENABLE`.
4. After clicking `ENABLE`, while still in the `YouTube Data API v3` section of the console, click `CREATE CREDENTIALS`.
5. Select `YouTube Data API v3` for the credential type and ensure `User data` is selected for access. Click `NEXT`.
6. Fill in the application information. For example, name the application `playlist-manager`, choose a user support email, and, optionally, an app logo image. Enter your email address under developer contact information. Click `SAVE AND CONTINUE`.
7. Click `ADD OR REMOVE SCOPES`, filter on `YouTube`, select the scopes `youtube.readonly`, `youtube`, and `youtubepartner`, and click `UPDATE`. Click `SAVE AND CONTINUE`.
8. Under OAuth Client ID, choose application type `Desktop app`, and name the client ID e.g. `playlist manager client`. Click `CREATE`.
9. Click `DOWNLOAD` and save the client id file to disk. (This is technically called a `client secret` file, although it's not particularly sensitive.)
10. After downloading the client id file, click `OAuth consent screen page`, click `ADD USERS`, and enter the email address(es) of users you want to authorise to use the application.

## Command line interface

WIP, but you can run the code like this:

cargo run -- &lt;playlist-id&gt; --client=/path/to/client_id.json

Use `--help` for more information.