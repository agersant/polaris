# Contributing

## Guidelines

While Polaris is free and open-source software, it is not open to code contributions. This still leaves room for a few avenues to help:
- Package Polaris for a Linux distribution
- Answer questions in the issue tracker
- Start or contribute to satellite projects
- Maintain your own fork

Contributions (documentation, issues, pull requests, etc.) making any use of AI/LLMs will be rejected.

## Compiling and running Polaris

1. [Install Rust](https://www.rust-lang.org/en-US/install.html) (stable toolchain)
2. Clone the polaris depot with this command: `git clone https://github.com/agersant/polaris.git`
3. You can now run compile and run polaris from the newly created directory with the command: `cargo run`

Polaris supports a few command line arguments which are useful during development:

- `-c some/config.toml` sets the location of the [configuration](/docs/CONFIGURATION.md) file.
- `--data some/path` sets the folder Polaris will use to store runtime data such as playlists, collection index and auth secrets.
- `-w some/path/to/web/dir` lets you point to the directory to be served as the web interface. You can find a suitable directory in your Polaris install (under `/web`), or from the [latest polaris-web release](https://github.com/agersant/polaris-web/releases/latest/download/web.zip).
- `-f` (on Linux) makes Polaris not fork into a separate process.

Putting it all together, a typical command to compile and run the program would be: `cargo run -- -w web -c test-config.toml`

While Polaris is running, access the web UI at [http://localhost:5050](http://localhost:5050).

## Running unit tests
Run `cargo test`!
