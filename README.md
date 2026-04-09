<div align="center">
  <h1><img src="res/readme/logo.png?raw=true"/></h1>

  [![Actions Status](https://github.com/agersant/polaris/workflows/Build/badge.svg)](https://github.com/agersant/polaris/actions)
  [![codecov](https://codecov.io/github/agersant/polaris/graph/badge.svg?token=EQqCmBEf2T)](https://codecov.io/github/agersant/polaris)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)

  ![Polaris Web UI](res/readme/web_ui.png?raw=true "Polaris Web UI")
</div>

# About

Polaris is a self-hosted music streaming server, to enjoy your music collection from any computer or mobile device. It is free and open-source software, without any kind of premium version.

The goals of this project are:
- 🔥 Exceptional performance and responsiveness
- 📚️ First-class support for large music collections (100,000+ songs)
- 📦️ Ease of installation, deployment and maintenance
- ✨ Beautiful user interface

# Try It Out!

On your desktop or laptop computer, check out the demo over at https://demo.polaris.stream featuring a selection of Creative Commons Music. The credentials to access this server are:

Username: `demo_user`  
Password: `demo_password`

# Features

- 🖥️ Runs on Windows, Linux, BSD, or through Docker
- 🔊 Support for `flac`, `mp3`, `mp4`, `mpc`, `ogg`, `opus`, `ape`, `wav` and `aiff` files
- 🌈 Dark mode variants and customizable color palette
- 💿️ Browse your music by album, artist or genre
- 📂 Browse your music as a file tree
- 🌊 Song audio-waveform visualization
- 🏷️ Support for multi-value fields in song metadata (eg. multiple artists per song)
- 🔍️ Powerful search functionality with per-field queries
- ⚙️ Plain-text configuration also editable with built-in UI
- 👥 Setup multiple users, each with their own playlists
- 📱 Listen to your music on the go:
  - Polaris Android ([Google Play Store](https://play.google.com/store/apps/details?id=agersant.polaris) · [F-Droid](https://f-droid.org/packages/agersant.polaris/) · [Repository](https://github.com/agersant/polaris-android))
  - Polarios ([App Store](https://apps.apple.com/app/polarios/id1662366309) · [Repository](https://gitlab.com/elise/Polarios)) [third-party]
  - Polarity ([Demo](https://www.hackster.io/hardcoder/polarity-a-music-player-for-polaris-8cd4eb) · [Repository](https://github.com/p-dial8891/Polarity)) [third-party]

# Installation

[Installation documentation](docs/SETUP.md)

[Streaming from remote devices](docs/DDNS.md)

[![Packaging status](https://repology.org/badge/vertical-allrepos/polaris-streaming.svg?columns=3)](https://repology.org/project/polaris-streaming/versions)

# Documentation

- 📒 [Changelog](CHANGELOG.md)
- 🔧 [Configuration](docs/CONFIGURATION.md)
- 👷 [Contribute to Polaris](docs/CONTRIBUTING.md)
- 🛟 [Maintenance Runbooks](docs/MAINTENANCE.md)

The Polaris server API is documented via [OpenAPI](https://demo.polaris.stream/api-docs/). Every installation of Polaris distributes this interactive documentation. To access it, open http://localhost:5050/api-docs/ in your browser on the machine running Polaris.

# Credits & License Information

Music featured in the demo installation:

- [Chris Zabriskie - Abandon Babylon](https://chriszabriskie.bandcamp.com/album/abandon-babylon) [(License)](https://creativecommons.org/licenses/by/3.0/)
- [Chris Zabriskie - Angie's Sunday Service](https://chriszabriskie.bandcamp.com/album/angies-sunday-service) [(License)](https://creativecommons.org/licenses/by/3.0/)
- [glaciære - pool water blue](https://steviasphere.bandcamp.com/album/pool-water-blue) [(License)](https://creativecommons.org/licenses/by/3.0/)
- [glaciære - light ripples](https://steviasphere.bandcamp.com/album/light-ripples) [(License)](https://creativecommons.org/licenses/by/3.0/)
- [Koresma South](https://koresma.bandcamp.com/album/south) [(License)](https://creativecommons.org/licenses/by-nc-sa/3.0/)
- [Pete Murphy - Essence EP](https://petemurphy.bandcamp.com/album/falling-down-the-fred-astaires-solo-jazz-piano) [(License)](https://creativecommons.org/licenses/by-nc-sa/3.0/)
- [Rameses B - Essence EP](https://ramesesb.bandcamp.com/album/essence-ep) [(License)](https://creativecommons.org/licenses/by-nc-nd/3.0/)
