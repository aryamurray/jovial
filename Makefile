.PHONY: build test check clippy fmt clean install

build:
	cargo build --workspace

test:
	cargo test --workspace

check:
	cargo check --workspace

clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clean:
	cargo clean

install:
	cargo install --path crates/jovial-cli

all: fmt clippy test build
