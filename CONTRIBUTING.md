# Compiling and Running Polaris

Compiling and running Polaris is very easy as it only depends on the Rust toolchain.

1. [Install Rust](https://www.rust-lang.org/en-US/install.html)
2. Clone the polaris depot with this command: `git clone --recursive https://github.com/agersant/polaris.git`
3. You can now run compile and run polaris from the newly created directory with the command: `cargo run`

Polaris supports a few command line arguments which are useful during development:

- `-w some/path/to/web/dir` lets you point to the directory to be served as the web interface. You'll probably want to point this to the `/web` directory of the polaris repository.
- `-d some/path/to/a/file.db` lets you manually choose where Polaris stores its configuration and music index (you can reuse the same database accross multiple runs)
- `-f` (on Linux) makes Polaris not fork into a separate process

Putting it all together, a typical command to compile and run the program would be: `cargo run -- -w web -d test/my.db`

While Polaris is running, access the web UI at [http://localhost:5050](http://localhost:5050).

# Running Unit Tests

That's the easy part, simply run `cargo test`!
