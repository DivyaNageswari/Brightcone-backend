# Needs GNU make. On Windows without make, run the commands inside each target by hand (see README).
-include .env
export

.PHONY: db-up db-down db-reset migrate run seed test lint fmt openapi \
        docker-migrate docker-seed docker-run docker-down docker-reset

db-up: ## start Postgres and the mail catcher
	docker compose up -d --wait postgres mailpit

db-down: ## stop services (data is kept)
	docker compose down

db-reset: ## DELETE all local data and start from an empty database
	docker compose down -v
	docker compose up -d --wait postgres mailpit
	$(MAKE) migrate

migrate: ## apply migrations as the app_migrator role
	DATABASE_URL="$(MIGRATION_DATABASE_URL)" cargo loco db migrate

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

# --- the same steps with no Rust on the host (Docker only) ---------------------
docker-migrate: ## apply migrations in Docker, as the app_migrator role
	docker compose --profile app run --rm migrate

docker-seed: ## create the demo accounts in Docker
	docker compose --profile app run --rm seed

docker-run: ## start the API in Docker on http://localhost:5150
	docker compose --profile app up api

docker-down: ## stop every container (data is kept)
	docker compose --profile app down

docker-reset: ## DELETE the database volume, then migrate from scratch
	docker compose --profile app down -v
	docker compose up -d --wait postgres mailpit
	$(MAKE) docker-migrate
