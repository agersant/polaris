# Contributing

## Guidelines

While Polaris is free and open-source software, it is not very open to code contributions. The reasons behind this are:
- Polaris is a hobby project. I don't want it to feel like my day job, where I do a lot of code reviews, mentoring and tech leadership.
- I am committed to maintaining this software for a very long time. I would rather maintain code that I mostly wrote myself.

This still leaves room for a few avenues to contribute:
- Help answering questions in the issue tracker.
- Package Polaris for a Linux distribution
- Documentation improvements or writing user guides.
- Satellite projects (eg. [docker-polaris](https://github.com/ogarcia/docker-polaris), [polarios](https://gitlab.com/elise/Polarios))
- Bug fixes.

For non-trivial new features, you are welcome to maintain a fork. If you need help finding your way around the code, feel free to open a [discussion thread](https://github.com/agersant/polaris/discussions).

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

That's the easy part, simply run `cargo test`!
