.PHONY: build install uninstall clean test check clippy

BINARY_NAME := ultradian-work
RELEASE_DIR := target/release

build:
	cargo build --release

install:
	./install.sh

uninstall:
	./install.sh --uninstall

clean:
	cargo clean
	rm -rf target/

test:
	cargo test

check:
	cargo check

clippy:
	cargo clippy --all-targets -- -D warnings

run:
	cargo run --release

dev:
	cargo run

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets -- -D warnings
