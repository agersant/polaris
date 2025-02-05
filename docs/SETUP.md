# Installation

## On Windows

1. Download the [latest installer](https://github.com/agersant/polaris/releases/latest) (you want the .msi file)
2. Run the installer
3. Launch Polaris from the start menu
4. In your web browser, access http://localhost:5050

## In a docker container

To run polaris from a Docker container, please follow instructions from the [docker-polaris](https://github.com/ogarcia/docker-polaris) repository.

## From source on Linux

### Dependencies

1. Install OpenSSL, SQLite and their respective headers (eg. `sudo apt-get install libsqlite3-dev libssl-dev`).
2. Install `binutils` and `pkg-config` (eg. `sudo apt-get install binutils pkg-config`).
2. Install the Rust compiler by executing `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` or using an [alternative method](https://www.rust-lang.org/en-US/install.html)

### Polaris installation
1. Download the [latest release]((https://github.com/agersant/polaris/releases/latest)) of Polaris (you want the .tar.gz file)
2. Extract the Polaris archive in a directory and open a terminal in that directory
3. To install Polaris within your home directory, execute `make install-xdg`. This installation follows the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html). You can use `make preview-xdg` to see which directories the install process would use.
4. If you prefer a system-wide install, execute `make install` (without the `-xdg` suffix). If you use `sudo` to perform such a system install, you may need the `-E` option so that your sudo user find the Rust binaries: `sudo -E make install`. This installation follows the [GNU Standard Installation Directories](https://www.gnu.org/prep/standards/html_node/Directory-Variables.html). You can use `make preview` to see which directories the install process would use.

From here, you might want to adjust your system to run Polaris on login using Systemd, Cron or whichever method your distribution endorses.

If you want to uninstall Polaris, execute `make uninstall-xdg` from the extracted archive's directory (or `make uninstall` if you made a system-wide install). This will delete all the files and directories listed above (including your configuration, playlists, etc.). If you customized the install process by specifying environment variables like `PREFIX`, make sure they are set to the same values when running the uninstall command.
