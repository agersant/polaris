#!/bin/sh
echo "Creating output directory"
mkdir -p release/tmp

echo "Copying package files"
cp -r web src Cargo.toml Cargo.lock res/unix/Makefile release/tmp

echo "Creating tarball"
POLARIS_VERSION=$(grep -m 1 ^version Cargo.toml | awk '{print $3}' | tr -d '"\r\n')
tar -zc -C release/tmp -f release/polaris-$POLARIS_VERSION.tar.gz .

echo "Cleaning up"
rm -rf release/tmp
