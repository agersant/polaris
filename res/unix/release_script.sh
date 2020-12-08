#!/bin/sh
echo "Creating output directory"
mkdir -p release/tmp/polaris

echo "Copying package files"
cp -r web docs/swagger src migrations test-data Cargo.toml Cargo.lock rust-toolchain res/unix/Makefile release/tmp/polaris

echo "Creating tarball"
tar -zc -C release/tmp -f release/polaris.tar.gz polaris

echo "Cleaning up"
rm -rf release/tmp
