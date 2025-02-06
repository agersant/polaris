# Configuration

Polaris configuration resides in a single text file whose format is documented below. You can use the Polaris web UI to modify the configuration, or write to it in any text editor. You may edit the configuration file while Polaris is running.

## Location

The location of the configuration file is always logged during Polaris startup. It is determined as follows:

- From the `--config` (or `-c`) CLI option if present. This option must point to the `.toml` file.
- If the CLI option is not specified, Polaris will look for a `polaris.toml` file, inside the directory specified by the `POLARIS_CONFIG_DIR` environment variable _at compilation time_. When using the Windows installer, this will be `%LOCALAPPDATA%/Permafrost/Polaris/polaris.toml`. When using the supplied Makefile, the default is either `/usr/local/etc/polaris` (for a system-wide installations), or `~/.config/polaris` (for a XDG installation).
- If `POLARIS_CONFIG_DIR` was not set when Polaris was compiled, it will default to `.` on Linux, and the `LOCALAPPDATA` location mentioned above on Windows. This behavior on Windows may change in future releases.

## Format

The configuration file uses the [TOML](https://toml.io/) format. Everything in the configuration file is optional and may be omitted (unless mentioned otherwise).

```toml
# Regular expression used to identify album art in files adjacent to an audio file
album_art_pattern = "Folder.(jpeg|jpg|png)"
# A URL Polaris will regularly make requests to in order to update Dynamic DNS
ddns_url = "https://example.com?token=foobar"

# Array of locations Polaris should scan to find music files
[[mount_dirs]]
# Directory to scan
source = "/home/example/music"
# User-facing name for this directory (must be unique)
name = "My Music 🎧️"

[[mount_dirs]]
source = "/mnt/example/more_music"
name = "Extra Music 🎵"

# Array of user accounts who can connect to the Polaris server
[[users]]
# Username for login
name = "example-user"
# If true, user will have access to all settings in the web UI
admin = true
# Plain text password for this user. Will be ignored if hashed_password is set. Polaris will never write to this field. For each user, at least one of initial_password and hashed_password must be set.
initial_password = "top-secret-password"
# Hashed and salted password for the user. Polaris will create this field if unset.
hashed_password = "$pbkdf2-sha256$i=10000,l=32$SI8LjK1KtvcawhgmWGJgRA$t9btMwhUTQ8r3vqI1xhArn19J7Jezyoi461fFjhZXGU"

[[users]]
name = "other-user"
admin = true
initial_password = "amospheric-strawberry64"
```

