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
2. Extract the polaris archive in a directory and open a terminal in that directory
3. If you would like to customize the directories used by the installation process, you can specify any number of the following environment variables: `PREFIX`, `EXEC_PREFIX`, `DATAROOTDIR`, `DATADIR`, `LOCALSTATEDIR` and `RUNSTATEDIR`. Refer to the [Make manual](https://www.gnu.org/software/make/manual/html_node/Directory-Variables.html#Directory-Variables) for details on how these are used.
4. Execute `make install` (this may take several minutes)

Using default install paths, the Polaris footprint on your system will be:
- The `polaris` executable in `/usr/local/bin`
- A handful of static files in `/usr/local/share/polaris`
- A database file containing your settings and music index in `usr/local/var/lib/polaris`
- A log file in `usr/local/var/log/polaris`
- Album art thumbnails in `usr/local/var/cache/polaris`
- A PID file in `usr/local/var/run/polaris`

From here, you might want to adjust your system to run Polaris on login using Systemd, Cron or whichever method your distribution endorses.

If you want to uninstall Polaris, execute `make uninstall` from the extracted archive's directory. This will delete all the files and directories listed above **including your Polaris database**. If you customized the install process by specifying environment variables like `PREFIX`, make sure they are set to the same values when running `make uninstall`.

### In a docker container

To run polaris from a Docker container, please follow instructions from the [docker-polaris](https://github.com/ogarcia/docker-polaris) repository.

## Test Run

- Start Polaris using the shortcut on your desktop (Windows) or by running the Polaris executable
- In your Web browser, access http://localhost:5050
- You will see a welcome page that will guide you through the Polaris configuration
