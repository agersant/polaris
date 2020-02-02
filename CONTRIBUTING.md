# Compiling and Running Polaris

Compiling and running Polaris is very easy as it only depends on the Rust toolchain.

1. [Install Rust](https://www.rust-lang.org/en-US/install.html)
2. Clone the polaris depot with this command: `git clone --recursive https://github.com/agersant/polaris.git`
3. You can now run compile and run polaris from the newly created directory with the command: `cargo run`

Polaris supports a few command line arguments which are useful during development:

- `-w some/path/to/web/dir` lets you point to the directory to be served as the web interface. You can find a suitable directory in your Polaris install (under `/web`), or from the [latest polaris-web release](https://github.com/agersant/polaris-web/releases/latest/download/web.zip).
- `-s some/path/to/swagger/dir` lets you point to the directory to be served as the swagger API documentation. You'll probably want to point this to the `/docs/swagger` directory of the polaris repository.
- `-d some/path/to/a/file.db` lets you manually choose where Polaris stores its configuration and music index (you can reuse the same database accross multiple runs).
- `-c some/config.toml` lets you use a configuration file to add content to the database. This can be useful if you frequently delete the database and would like to automate the first time flow. The configuration format is not documented but can be inferred by looking at the `Config` struct in `config.rs`.
- `-f` (on Linux) makes Polaris not fork into a separate process.

Putting it all together, a typical command to compile and run the program would be: `cargo run -- -w web -s docs/swagger -d test/my.db`

While Polaris is running, access the web UI at [http://localhost:5050](http://localhost:5050).

# Running Unit Tests

That's the easy part, simply run `cargo test`!
