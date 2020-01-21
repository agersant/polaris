#!/bin/sh
echo "Creating output directory"
mkdir -p release/tmp/polaris

echo "Copying package files"
cp -r web docs/swagger src migrations Cargo.toml Cargo.lock res/unix/Makefile release/tmp/polaris

echo "Creating tarball"
POLARIS_VERSION=$(grep -m 1 ^version Cargo.toml | awk '{print $3}' | tr -d '"\r\n')
tar -zc -C release/tmp -f release/Polaris_$POLARIS_VERSION.tar.gz polaris

echo "Cleaning up"
rm -rf release/tmp
