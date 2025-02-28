# Changelog

## Unreleased Changes

- Fixed a typo in the log message that is written after applying configuration changes. (thanks @luzpaz)
- Improved performance of indexing m4a files (contribution by @saecki)

## Polaris 0.15.0

### Server

- Added support for browsing the music collection by metadata (by artist, by genre, etc.).
- Added support for multi-value metadata for the following song fields: `artist`, `album artist`, `composer`, `genre`, `label` and `lyricist`.
- Added support for structured search query syntax.
- Added capability to extract audio waveform data.
- Configuration data (user credentials, music directories, etc.) is now stored in a plain-text file which Polaris can read and write to.
- ⚠️ The configuration format is now ([documented](docs/CONFIGURATION.md)) and slightly simpler than in previous versions.
- Persistent data, such as playlists, is now saved in a directory that may be configured with the `--data` CLI option or the `POLARIS_DATA_DIR` environment variable.
- ⚠️ Upon first launch, configuration data and playlists will be migrated from the Polaris 0.14.0 database into their new homes. After successful migration, the old database file will be deleted and the server will finally start. This migration functionality will be removed in future Polaris versions.
- Collection scans are now automatically triggered when configuration changes or files are added/removed.
- ⚠️ Dynamic DNS now works with any provider that supports updates over HTTP without header-based auth. This means YDNS is no longer an option, and you need to input a new URL for DDNS updates.
- ⚠️ Removed last.fm integration due to maintenance concerns (abandoned libraries, broken account linking) and mismatch with project goals.
- Removed periodic collection scans.

### Web client

- Every page has been updated to a new visual style.
- The file browser is now displayed as an interactive tree on a single page.
- The file browser now supports common navigation keyboard shortcuts.
- The file browser now supports jumping to a visible file or folder by typing the start of its name.
- The file browser now omits the top-level directory when only one music folder has been configured.
- The current playlist now has two display modes: compact or with album art.
- Songs in the current playlist can now be selected and re-ordered with the mouse.
- Added a button to display statistics about the current playlist.
- Added new pages to browse the music collection by genre.
- Added new pages to browse the music collection by artist.
- Added a new page to browse the music collection by album.
- The Recently Added Albums and Random Albums pages now distinguish albums by file metadata instead of file path.
- When navigating back to the Random Albums page, the shuffle ordering is now preserved.
- The current playlist now supports common navigation keyboard shortcuts.
- The seekbar for the current song being played has been replaced with a waveform visualization.
- The title of the current song in the player can be clicked to display its metadata
- Improved responsiveness when queuing large amounts of songs at once.
- The `Settings > Collection` page now shows the current status of collection scanning.
- Theme preferences have been reset and are now stored client-side.
- Accent color is now configured as a saturation multiplier and base hue, which are used to generate a full color ramp.

### API

- API version is now 8.0.
- Documentation is now served under `/api-docs` instead of `/swagger` (eg. `http://localhost:5050/api-docs`)
- Clients are now expected to send their preferred API major version in a `Accept-Version` header. Omitting this currently defaults to `7`, but will become an error in future Polaris releases. Support for API version 7 will be removed entirely in a future release.
- Most API responses now support gzip compression.
- The response format of the `/browse`, `/flatten`, `/get_playlist`, `/search/<query>` endpoints has been modified to accommodate large lists.
- Added new endpoints to query albums and artists.
- The `/random` and `/recent` albums are deprecated in favor of `/albums/random` and `/albums/recent`. These endpoints now have optional parameters for RNG seeding and pagination.
- The `/search/<query>` endpoint now requires a non-empty query (`/search/` now returns HTTP status code 404, regardless of API version).
- The `/search/<query>` endpoint now supports per-field queries and boolean combinators.
- The `/thumbnail` endpoint supports a new size labeled `tiny`, which returns 40x40px images.
- Added a new `/get_songs` endpoint which returns song metadata in bulk.
- Added a new `/peaks` endpoint which returns audio signal peaks that can be used to draw waveform visualizations.
- Added a new `/index_status` endpoint which returns the status of music collection scans.
- Removed the `/config` and `/preferences` API endpoints.
- Removed the `/ddns` API endpoints, merged into the existing `/settings` endpoints.

## Polaris 0.14.3

### Server

- Fixed a build error (https://github.com/rust-lang/rust/issues/127343) with recent versions of the Rust compiler (thanks @pbsds) 
- Added support for m4b audio files (thanks @duydl)

## Polaris 0.14.2

### Server

- Fixed a startup error in Windows packaged builds

## Polaris 0.14.1

### Server

- Fixed compilation issue when using musl toolchains
- Log messages that DDNS is not setup have been downgraded to debug level

### Web client

- Fixed a bug where non-ASCII files or directories were not always alphabetically sorted (thanks @dechamps)
- Fixed a bug where after linking a last.fm account, clicking the account name would not link to the expected page

## Polaris 0.14.0

### General

- Changes are now documented in `CHANGELOG.md` instead of inside individual Github releases

### Server

- API version is now 7.0
- ⚠️ Removed support for authentication via cookies (deprecated in Polaris 0.13.0)
- ⚠️ Removed support for authentication via the `Basic` scheme when using the HTTP `Authorization` header (deprecated in Polaris 0.13.0)
- Fixed a bug where all music sources would be deleted when trying to add sources with duplicate names
- Additional metadata fields are now indexed: lyricist, composer, genre and label (thanks @pmphfm)
- Endpoints returning thumbnail images or audio files no longer use HTTP `content-encoding`
- When indexing files with ID3v2 tags, the "Original Date Released" frame can now be used to populate the year associated with a song
- The `/thumbnail` endpoint now supports an optional parameter for small/large/native image sizing. (thanks @Saecki)
- Log file now contain more details about the cause of failed HTTP requests (3xx, 4xx, 5xx)
- Startup failures now generate clearer error messages

### Web client

- Volume slider now applies non-linearly
- Artist names are now displayed in the Random Albums and Recent Albums pages

## Polaris 0.13.5

### Server

- Added support for AIFF and WAVE files (thanks @gahag)

### Web Client

- Improved performance when scrolling large playlists
- Fixed display and playback issues when a song was used multiple times in a playlist
- Playlist duration can now display number of days
- Fixed a bug where the playlist panel could have blank space in very tall browser windows
- Major dependencies updates

## Polaris 0.13.4

### Server

Adjustments to logging behavior.

On Linux:

- Running without `-f` emits a log file
- Running with `-f` and no `--log` option does not emit a log file
- Running with `-f` and `--log` option emits a log file

On Windows:

- Running with UI feature (`polaris.exe` in releases) emits a log file
- Running without UI feature (`polaris-cli.exe` in releases) and no --log option does not emit a log file
- Running without UI feature (`polaris-cli.exe` in releases) and --log option emits a log file

## Polaris 0.13.3

### Server

- Fixed a bug where music that is no longer on disk was still considered in the collection, even after re-indexing
- On Windows, Polaris now creates a log file
- On Linux, Polaris now creates a log file, even when running with the -f option

## Polaris 0.13.2

### Web client

- Fixed a bug where it was not possible to view or edit which users have administrator rights
- Fixed a bug where, in some cases, drag and dropping a specific disc from an album would not queue the entire disc

## Polaris 0.13.1

### Server

- Fixed a bug where the Windows installer would create unusable installations. #122

## Polaris 0.13.0

### API changes

- Bumped API version number to 6.0.
- Added new endpoints to manage users, mount points and settings more granularly.
- Added support for authenticating via bearer tokens generated by the /auth endpoint. These token can be submitted via Bearer HTTP Authorization headers, or as a URL parameters (`?auth_token=…`).
- Authentication using cookies or Basic HTTP Authorization headers is deprecated and will be removed in a future revision.
- Authentication cookies no longer expire after 24 hours. The newly added bearer tokens also have no expiration date.
- Last.fm account linking now requires a short-lived auth token obtain from the newly added `lastfm/link_token' endpoint.

Server

- ⚠️Breaking change⚠️ If you use a config file, the `reindex_every_n_seconds` and `album_art_pattern` fields must now be in a [settings] section.
- ⚠️Breaking change⚠️ The installation process on Linux has changed a lot. See the README for updated installation instructions. A summary of the changes is available [here](https://github.com/ogarcia/docker-polaris/issues/2).
- Embedded album art is now supported for mp3, flac and m4a files (thanks @Saecki).
- OPUS files can now be indexed and streamed (thanks @zaethan).
- APE files can now be indexed and streamed.
- The collection indexer has been rewritten for better performance. This also fixed an issue where on some machines, the web client would be unusable while indexing (thanks @inicola for the code reviews).
- Thumbnail generation is now slightly faster, and works with more pixel formats (notably RGBA16).
- Polaris now uses actix-web instead or rocket. This change fixes numerous performance and stability issues.
- Sqlite is now bundled by default when building Polaris and was removed from the list of prerequisites. This can be controlled with the `bundle-sqlite` feature flag when compiling Polaris.
- The default album art pattern now includes the jpeg extension in addition to jpg.
- Album art patterns are now case insensitive.

Web client

- ⚠️Breaking change⚠️ Your current playlist will appear broken after this update. Please clear the current playlist using the trash can icon. Saved playlists are not affected.
- Added a logout button.
- Reworked interface for managing user accounts.
- Added a shuffle button to randomly re-order the content of the current playlist.
- The total duration of the current playlist is now displayed.
- Audio output can now be toggled on/off by clicking the volume icon.
- Individual discs from multi-disc albums can now be dragged into the playlist.
- When browsing to an album, songs are now displayed and queued in filepath order.
- Fixed a bug where albums could not be dragged from the random or recent views.
- Fixed a bug where directories with a # sign in their name could not be browsed to.

## Polaris 0.12.0

### Server

- Library indexing speed is now significantly faster
- When indexing files that have malformed ID3 tags, information preceding the error will no longer be discarded
- Deleted users can no longer make requests using an existing session
- When using a config file, existing users, mounts points and DDNS settings are no longer removed before applying the configuration
- When using a config file to create users, blank usernames are now ignored
- Improved architecture and added more unit tests

API Changes

- API version number bumped to 4.0
- The auth endpoint now returns HTTP cookies instead of a JSON response
- Client requests to update Last.fm status no longer return an error if no Last.fm account is associated with the user
- The thumbnail endpoint now supports an option to disable padding to a square image

Web client

- The web client now uses Vue instead of Riot as its UI framework
- Added support for theming

## Polaris 0.11.0

### Server

- Compatible with current versions of the Rust nightly compiler
- Fixed a rare crash when indexing corrupted mp3 files
- On Linux, Polaris now notifies systemd after starting up
- Release tarball for Linux version now includes a top-level directory
- User sessions no longer break across server restarts (more improvements still to do on this: #36)
- ⚠️ Breaking change: due to improvements in Polaris credentials management, you will have to re-create your users and playlists after upgrading to this version. If you want to preserve your playlists, you can use a program like DB Browser for SQLite to back up your playlists (from db.sqlite within your Polaris installation directory) and restore them after you re-create users with the same names.

### Web client

- Song durations are now listed when available
- Fixed a bug where clicking on breadcrumbs did not always work when the Polaris server is hosted on Windows
- Current track info now shows in browser tab title
- Fixed a semi-rare bug where indexing would not start during initial setup flow
- Improved handling of untagged songs
- Fixed a bug where playlist had padding in Chrome
- Fixed a bug where folder icons did not render on some systems

Thank you to @lnicola for working on most of the server changes!

## Polaris 0.10.0

### Server

- Polaris servers now ship with an interactive API documentation, available at http://localhost:5050/swagger
- When using a prefix URL in Polaris config files, a / will no longer be added automatically at the end of the prefix

### Web client

- Automatically bring up player panel when songs are queued
- Fixed a bug where songs were not always correctly sorted by track number in browser panel
- Fixed a bug where some button hitboxes didn't match their visuals

## Polaris 0.9.0

### Server

- Rewrote all endpoints and server setup using Rocket instead of Iron
- Fixed a bug where special characters in URL to collection folders were not handled correctly (bumped API version number)
- Server API is now unit tested
- Fixed a bug where lastFM integration endpoints did not work
- ⚠️ Compiling Polaris now requires the nightly version of the Rust compiler

### Web client

- Encode special characters in URL to collection folders

## Polaris 0.8.0

### Server

- Added new API endpoints for search
- Added new API endpoints for Last.fm integration
- Thumbnails are now stored as .jpg images instead of .png
- Duration of some audio files is now being indexed
- On Linux when running as a forking process, a .pid file will be written
- Fixed a bug where usernames were inserted in session even after failed authentication

### Web client

- Added search panel
- Added settings tab to link Last.fm account

## Polaris 0.7.1

### Server

- Added support for prefix_url option in configuration files
- Improved performance of thumbnail creation

## Polaris 0.7.0

### Server

- Added support for the Partial-Content HTTP header when serving music, this fixes several streaming/seeking issues when using the web client (especially in Chrome)
- New API endpoints for playlist management
- New command line argument (-p) to run on a custom port (contribution from @jxs)
- New command line argument (-f) to run in foreground on Linux (contribution from @jxs)
- Fixed a bug where tracks were queued out of order
- Updated program icon on Windows

Web client

- Added support for playlists
- Added a button to to queue the current directory (thanks @jxs)

## Polaris 0.6.0

### Server

- Internal improvements to database management (now using Diesel)
- Configuration settings are now stored in the database, polaris.toml config files are no longer loaded by default
- Added API endpoints to read and write configuration
- User passwords are now encrypted in storage
- Fixed a bug where results of api/browse were not sorted correctly

Web client

- Settings can now be edited from the web UI
- Collection re-index can now be triggered from the web UI
- Added initial setup configuration flow to help set up first user and mount point
- Visual changes

## Polaris 0.5.1

This is a minor release, pushing quite a bit of internal cleanup in the wild.

Server

- Removed OpenSSL dependency on Windows
- No longer send a HTTP cookie after authentication

## Polaris 0.5.0

This releases adds Linux support and a variety of improvements to the web client.

### Server

- Added Linux support
- Moved location of configuration file on Windows to `%appdata%\Permafrost\Polaris\polaris.toml`

### Web client

- Performance improvements from upgrading RiotJS to 3.4.4 (from 2.6.2)
- Added support for browsing random and recently added albums
- Minor visual changes (colors, whitespace, etc.)
- Updated favicon
- Fixed a bug where songs containing special characters in their title would not play
- Persist playlist and player state across sessions

## Polaris 0.4.0

This release adds new features supporting the development of polaris-android.

### Server

- Added API endpoint to pull recently added albums
- Added support for the Authorization HTTP header (in addition to the existing /auth API endpoint)

## Polaris 0.3.0

This release is an intermediate release addressing issues with the installation process and updating internals.

### General

- Fixed missing OpenSSL DLL in Windows installer (fixes Issue #3)
- Split every file into an individual installer component

### Server

- Added API endpoint to pull random albums
- Upgraded dependencies
- Added unit tests to indexing and metadata decoding

### Web client

- Web interface playlist now displays more tracks (enough to fill a 4k monitor at normal font size)

## Polaris 0.2.0

This release is focused on polish and performance, solidifying the basics that were put together in version 0.1.0. Here are the major changes:

### General

- Polaris now has a project logo
- Windows installer now supports upgrading an existing install (from 0.2.0 to higher - versions)
- Added support for multi-disc albums

### Server

- Major performance improvements to /browse and /flatten API requests (up to 1000x - faster for large requests)
- Added API endpoint for version number
- Album covers are now served as thumbnails rather than at source size
- Moved configuration file outside of /Program Files
- Added support for Ogg Vorbis, FLAC and APE metadata
- Fixed a bug where most albums didn't show an artist name
- Fixed a bug where uppercase extensions were not recognized
- Upgraded compiler to Rust 1.13

### Web client

- Complete visual overhaul of the Polaris web client
- Performance improvements for handling large playlist in Polaris web client
- Added error messages when playing songs in unsupported formats

## Polaris 0.1.0

This is the very first Polaris release, celebrating the minimum viable product!

Features in this release:

- Server application with Windows Installer
- Support for multiple users
- Support for serving custom music directories
- Support for custom album art pattern matching
- Support for broadcasting IP to YDNS
- Web UI to browse collection, manage playlist and listen to music
