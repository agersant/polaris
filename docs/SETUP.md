# Getting Started

## Requirements

One of the following:
- Windows 7 or newer
- Linux (any reasonably modern distribution should do)

### Windows
1. Download the [latest installer](https://github.com/agersant/polaris/releases/latest) (you want the .msi file)
2. Run the installer
3. That's it, you're done!

You can now start Polaris from the start menu or from your desktop, Polaris will also start automatically next time you restart your computer. You can tell when Polaris is running by its icon in the notification area (near the clock and volume controls).

### Linux

#### Dependencies

1. Install OpenSSL, SQLite and their headers, and some development tools. These are available from your distribution's package manager. For instance on Ubuntu, execute `sudo apt-get install binutils pkg-config libssl-dev libsqlite3-dev`
2. Install the nightly Rust compiler by executing `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` or using an [alternative method](https://www.rust-lang.org/en-US/install.html)

#### Polaris installation
1. Download the [latest release]((https://github.com/agersant/polaris/releases/latest)) of Polaris (you want the .tar.gz file)
2. Extract the Polaris archive in a directory and open a terminal in that directory
3. To install Polaris within your home directory, execute `make install-xdg`. This installation follows the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html). You can use `make preview-xdg` to see which directories the install process would use.
4. If you prefer a system-wide install, execute `make install` (without the `-xdg` suffix). If you use `sudo` to perform such a system install, you may need the `-E` option so that your sudo user find the Rust binaries: `sudo -E make install`. This installation follows the [GNU Standard Installation Directories](https://www.gnu.org/prep/standards/html_node/Directory-Variables.html). You can use `make preview` to see which directories the install process would use.


From here, you might want to adjust your system to run Polaris on login using Systemd, Cron or whichever method your distribution endorses.

If you want to uninstall Polaris, execute `make uninstall-xdg` from the extracted archive's directory (or `make uninstall` if you made a system-wide install). This will delete all the files and directories listed above **including your Polaris database**. If you customized the install process by specifying environment variables like `PREFIX`, make sure they are set to the same values when running the uninstall command.

### In a docker container

To run polaris from a Docker container, please follow instructions from the [docker-polaris](https://github.com/ogarcia/docker-polaris) repository.

## Test Run

- Start Polaris using the shortcut on your desktop (Windows) or by running the Polaris executable
- In your Web browser, access http://localhost:5050
- You will see a welcome page that will guide you through the Polaris configuration
