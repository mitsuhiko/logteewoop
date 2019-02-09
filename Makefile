all: test

build:
	@cargo build

doc:
	@cargo doc

test: cargotest

cargotest:
	@cargo test

format:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt

format-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt -- --check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy

update-readme:
	@cargo readme > README.md

.PHONY: all build doc test cargotest format format-check lint update-readme
