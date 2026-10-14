# VisualAssembler: every project command. Thin wrapper; all logic lives in cargo / cargo xtask.
# Each recipe line is a single command with no shell syntax, so it behaves the same whether
# GNU make uses sh (Git for Windows, Ubuntu) or cmd.exe (Windows without sh on PATH).
# Normative content: docs/specs/m0-foundations.md section 7, amended by sections 17.5 and 18.7
# (security target). Comments may be added; values must not change.

CARGO    ?= cargo
SCENARIO ?= all
RENDERER ?= wgpu
RUNS     ?=

.DEFAULT_GOAL := help
.PHONY: help install hooks dev run test lint format security docs build clean bench ci

help:
	$(info VisualAssembler - make targets)
	$(info   install   install the pinned toolchain and cargo-deny 0.20.2 then fetch crates)
	$(info   hooks     install the commit-msg git hook)
	$(info   dev       run the GUI from a debug build)
	$(info   run       run the GUI from a release build)
	$(info   test      run every test of the workspace)
	$(info   lint      check formatting and run clippy with warnings as errors)
	$(info   format    format all Rust code)
	$(info   security  run cargo-deny on advisories bans licenses and sources)
	$(info   docs      regenerate AGENCEMENT.md and verify the documentation ritual)
	$(info   build     release build of the GUI and the CLI)
	$(info   clean     remove build artifacts)
	$(info   bench     measure the GUI - SCENARIO=startup|idle|all RENDERER=wgpu|glow|all RUNS=n)
	$(info   ci        run the CI checks locally - fmt clippy layering tests)

install:
	rustup toolchain install
	$(CARGO) install cargo-deny --version 0.20.2 --locked
	$(CARGO) fetch

hooks:
	$(CARGO) xtask hooks

dev:
	$(CARGO) run -p visual-assembler

run:
	$(CARGO) run --release -p visual-assembler

test:
	$(CARGO) test --workspace --all-features

lint:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

format:
	$(CARGO) fmt --all

# Explicit form (section 17.5): a stray root deny.toml can never shadow the policy.
# --locked (section 18.7): a stale Cargo.lock is an error instead of being re-resolved.
security:
	$(CARGO) deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check

docs:
	$(CARGO) xtask docs

build:
	$(CARGO) build --release -p visual-assembler -p va

clean:
	$(CARGO) clean

bench:
	$(CARGO) xtask bench $(SCENARIO) --renderer $(RENDERER) $(if $(RUNS),--runs $(RUNS))

ci:
	$(CARGO) xtask ci
