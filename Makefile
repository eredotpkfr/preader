SHELL=/bin/bash

.PHONY: all

all:
	book-build \
	book-test \
	cargo-build \
	cargo-check \
	cargo-clean \
	cargo-doc \
	cargo-doc-rs \
	cargo-doc-test \
	cargo-fix \
	cargo-machete \
	cargo-nextest \
	cargo-test \
	cargo-udeps \
	clippy \
	coverage \
	deny \
	develop \
	install-cargo-clippy \
	install-cargo-deny \
	install-cargo-doc-rs \
	install-cargo-llvm-cov \
	install-cargo-machete \
	install-cargo-mdbook \
	install-cargo-nextest \
	install-cargo-tools \
	install-cargo-udeps \
	install-nightly-toolchain \
	install-pre-commit-hooks \
	install-pre-commit-linux \
	install-pre-commit-mac \
	install-uv-linux \
	install-uv-mac \
	live-book \
	rustfmt \
	rustfmt-check \
	rustup \
	shell \
	update-pre-commit-hooks \
	uv-create-venv \

book-build:
	@mdbook build book
book-test:
	@mdbook test book
cargo-build:
	@cargo build
cargo-check:
	@cargo check
cargo-clean:
	@cargo clean
cargo-doc:
	@cargo doc
cargo-doc-rs:
	@cargo +nightly docs-rs
cargo-doc-test:
	@cargo test --doc
cargo-fix:
	@cargo fix --allow-dirty --allow-staged
cargo-machete:
	@cargo machete
cargo-nextest:
	@cargo nextest run
cargo-test:
	@cargo test
cargo-udeps:
	@cargo +nightly udeps
clippy:
	@cargo clippy --all-targets --all-features
coverage:
	@cargo +nightly llvm-cov \
		--all-features \
		--workspace \
		--doctests \
		--html \
		--open
deny:
	@cargo deny --all-features --log-level error check
develop:
	@uv run --no-sync --no-cache maturin develop
install-cargo-clippy:
	@rustup component add clippy
install-cargo-deny:
	@cargo install cargo-deny --locked
install-cargo-doc-rs:
	@cargo install cargo-docs-rs
install-cargo-llvm-cov:
	@cargo install cargo-llvm-cov --locked
install-cargo-machete:
	@cargo install cargo-machete --locked
install-cargo-mdbook:
	@cargo install mdbook
install-cargo-nextest:
	@cargo install cargo-nextest --locked
install-cargo-tools:
	install-cargo-clippy \
	install-cargo-deny \
	install-cargo-doc-rs \
	install-cargo-llvm-cov \
	install-cargo-machete \
	install-cargo-nextest \
	install-cargo-udeps \
	install-nightly-toolchain \
install-cargo-udeps:
	@cargo install cargo-udeps --locked
install-nightly-toolchain:
	@rustup toolchain install nightly
install-pre-commit-hooks:
	@pre-commit install --install-hooks
	@pre-commit install --hook-type commit-msg --install-hooks
install-pre-commit-linux:
	@sudo apt install pre-commit
install-pre-commit-mac:
	@brew install pre-commit
install-uv-linux:
	@curl -LsSf https://astral.sh/uv/install.sh | sh
install-uv-mac:
	@brew install uv
live-book: book-test
	@mdbook serve book
rustfmt: cargo-fix
	@cargo +nightly fmt --all
rustfmt-check:
	@cargo +nightly fmt --all -- --check
rustup:
	@rustup self update
	@rustup update
shell: develop
	@uv run --no-sync --no-cache python3
update-pre-commit-hooks:
	@pre-commit autoupdate
uv-create-venv:
	@uv venv --python $(shell python3 --version | cut -d" " -f2)
