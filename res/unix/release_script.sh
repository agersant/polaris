#!/bin/sh
echo "Creating output directory"
mkdir -p release/tmp/polaris

echo "Copying package files"
cp -r web src test-data build.rs Cargo.toml Cargo.lock rust-toolchain.toml res/unix/Makefile release/tmp/polaris

echo "Creating tarball"
tar -zc -C release/tmp -f release/polaris.tar.gz polaris

echo "Cleaning up"
rm -rf release/tmp
