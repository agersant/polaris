[![Actions Status](https://github.com/agersant/polaris/workflows/Build/badge.svg)](https://github.com/agersant/polaris/actions)
[![codecov.io](http://codecov.io/github/agersant/polaris/branch/master/graphs/badge.svg)](http://codecov.io/github/agersant/polaris)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)

<img src="res/readme/logo.png?raw=true"/>
Polaris is a music streaming application, designed to let you enjoy your music collection from any computer or mobile device. Polaris works by streaming your music directly from your own computer, without uploading it to a third-party. It is free and open-source software, without any kind of premium version. The only requirement is that your computer stays on while it streams music!

## Features
- Optimized for large music collections
- Can run on Windows, Linux, BSD, or through Docker
- Listen to your music on the web or using the [Polaris Android](https://github.com/agersant/polaris-android) app
- Easy to setup and configure via the built-in web UI
- Support for `flac`, `mp3`, `mp4`, `mpc`, `ogg`, `opus`, `ape`, `wav` and `aiff` files
- Support for album art images
- [Last.fm](https://www.last.fm) scrobbling
- Color themes
- Restrict access to your music collection with user accounts

## Tutorials

- [Getting Started](docs/SETUP.md)
- [Streaming From Remote Devices](docs/DDNS.md)

## Screenshots

![Polaris Web UI](res/readme/web_ui.png?raw=true "Polaris Web UI")
![Polaris Web UI Dark Mode](res/readme/dark_mode.png?raw=true "Polaris Web UI")

## Documentation

- [Contribute to Polaris](docs/CONTRIBUTING.md)
- [Maintenance Runbooks](docs/MAINTENANCE.md)

### API Documentation
The Polaris server API is documented via [Swagger](https://agersant.github.io/polaris/swagger). Please note that this Swagger page does not point to a live Polaris server so the `Try it out` buttons are not expected to work.
Every installation of Polaris also distributes this documentation, with the ability to use the `Try it out` buttons. To access it, simply open http://localhost:5050/swagger/ in your browser on the machine running Polaris.

Feel free to open Github issues or Pull Requests if clarifications are needed.
