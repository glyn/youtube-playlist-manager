# YouTube playlist manager

You can use this command line application to manage a YouTube playlist, print out its contents, sort it into a reasonable order, and remove unwanted entries.

The application is intended for use with a YouTube playlist which contains both streamed videos and scheduled streams. Sorting the playlist will move all the streamed videos to the top, in reverse chronological order (newest first). This makes it simple for the audience to "catch up" with the latest video in the playlist. If a new stream is created from an existing video in the playlist, the new stream is added to the same playlist, which can then be sorted if necessary using this application.

Removing unwanted entries removes older streamed videos leaving at most a given number present. It also removes invalid videos, such as any which have been deleted.

When you run the application, it will occasionally launch a web browser to gain the necessary authorisation to access or modify the playlist. Choose a suitable account and follow the instructions in the browser to give the application
permission. Note that the browser will be launched once to read the playlist and again to modify the playlist. The permissions will be cached on disk and reused, but may expire, in which case the application will launch the web browse again. To make all this possible, you need to create and download a client ID file, as described in the next section.

## Authentication and authorisation

The application uses an OAuth2 client ID to access the YouTube API. Read the [instructions](https://developers.google.com/identity/protocols/oauth2#installed) for how to create a client ID and download a file containing the client ID and its private key (follow the steps below). Specify the path to this client ID file using the `--client` switch.

Steps:
1. Open the [Google API Console](https://console.developers.google.com/) and ensure `Dashboard` is selected in the navigation list on the left.
2. Use the project drop-down at the top to create a new project called, for example, `playlist-manager`.
3. Click `+ ENABLE APIS AND SERVICES`, search for `YouTube`, select `YouTube Data API v3`, and click `ENABLE`.
4. After clicking `ENABLE`, while still in the `YouTube Data API v3` section of the console, click `CREATE CREDENTIALS`.
5. Select `YouTube Data API v3` for the credential type and ensure `User data` is selected for access. Click `NEXT`.
6. Fill in the application information. For example, name the application `playlist-manager`, choose a user support email, and, optionally, an app logo image. Enter your email address under developer contact information. Click `SAVE AND CONTINUE`.
7. Click `ADD OR REMOVE SCOPES`, filter on `YouTube`, select the scopes `youtube.readonly`, `youtube`, and `youtubepartner`, and click `UPDATE`. Click `SAVE AND CONTINUE`.
8. Under OAuth Client ID, choose application type `Desktop app`, and name the client ID e.g. `playlist manager client`. Click `CREATE`.
9. Click `DOWNLOAD` and save the client ID file to disk. (This is technically called a `client secret` file, although it's not particularly sensitive.)
10. After downloading the client ID file, click `OAuth consent screen page`, click `ADD USERS`, and enter the email address(es) of users you want to authorise to use the application.

## Command line interface

Run the application like this in a terminal on macOS or Linux:

```
playlist-manager <playlist-id> --client=/path/to/client_id.json
```

and like this in a command prompt or powershell on Windows:

```
playlist-manager.exe <playlist-id> --client=/path/to/client_id.json
```

Add the parameter `--help` for more information on the other parameters you can specify.

## Manual alternative

You can use the YouTube web interface to edit a playlist and manually drag its contents into the desired order. You can remove excess entries. But you can't remove certain invalid videos, such as those which have been deleted, since these are hidden in the web interface.

## Developer information

The application is written in Rust. Install Rust using [`rustup`](https://rustup.rs/).

To run the unit tests, clone this repository and issue the following command from the root directory of the clone:
```
cargo test
```

To create a release, push a new tag of the form `vn.n.n` (e.g. `v0.1.2`) to this repository and a release will be created and binaries compiled for Windows, macOS, and Linux.

Pull requests are welcome.
