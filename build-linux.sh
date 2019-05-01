#!/bin/bash
docker run --rm \
  -v /Users/notid/dev/playground/rust-cli/filesets:/app \
  -w /app/edit-server \
  rustlang/rust:nightly \
  cargo build --release --target=x86_64-unknown-linux-gnu

