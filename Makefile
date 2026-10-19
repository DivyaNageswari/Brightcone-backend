# Needs GNU make. On Windows without make, run the commands inside each target by hand (see README).
-include .env
export

.PHONY: run seed test lint fmt openapi line-limit

run: ## start the API on http://localhost:5150 with worker
	cargo loco start --server-and-worker

seed: ## create the 14 demo accounts; prints each password ONCE
	cargo loco task seed_demo

test: ## run all tests against brightcone_test
	cargo test --all-features

lint: ## format check + clippy
	cargo fmt --all --check
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## format code
	cargo fmt --all

openapi: ## regenerate openapi.json after changing the API
	UPDATE_OPENAPI=1 cargo test --test openapi

line-limit: ## enforce the 400-line Rust source file limit
	rustfmt --edition 2021 --check scripts/check_line_limits.rs
	mkdir -p target
	rustc --edition=2021 scripts/check_line_limits.rs -o target/check_line_limits.exe
	target/check_line_limits.exe
