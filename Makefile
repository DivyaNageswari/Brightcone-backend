# Needs GNU make. On Windows without make, run the commands inside each target by hand (see README).
-include .env
export

.PHONY: db-up db-down db-reset run seed test lint fmt openapi line-limit \
        docker-seed docker-run docker-down docker-reset

db-up: ## start Postgres and the mail catcher
	docker compose up -d --wait postgres mailpit

db-down: ## stop services (data is kept)
	docker compose down

db-reset: ## DELETE the local database volume; startup sync recreates schema
	docker compose down -v
	docker compose up -d --wait postgres mailpit

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

# --- the same steps with no Rust on the host (Docker only) ---------------------
docker-seed: ## create the demo accounts in Docker
	docker compose --profile app run --rm seed

docker-run: ## start the API in Docker on http://localhost:5150
	docker compose --profile app up api

docker-down: ## stop every container (data is kept)
	docker compose --profile app down

docker-reset: ## DELETE the database volume; API startup recreates schema
	docker compose --profile app down -v
	docker compose up -d --wait postgres mailpit
