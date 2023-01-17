!/bin/sh

rustup default 1.64.0
cargo clippy -- -D warnings

rustup default stable
cargo clippy -- -D warnings

rustup default nightly
cargo clippy -- -D warnings
