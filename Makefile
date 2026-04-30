.PHONY: help build test deploy-devnet validator stop shell clean image

help: ## show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

image: ## build the anchor docker image
	docker compose build anchor

build: ## anchor build (compiles the program inside docker)
	docker compose run --rm anchor anchor build

test: ## bankrun integration tests (in-process, no validator, runs on any cpu)
	pnpm test

validator: ## run a long-running solana-test-validator on :8899
	docker compose up -d validator
	@echo "validator running on http://localhost:8899"

stop: ## stop the validator
	docker compose stop validator

shell: ## drop into the anchor container shell
	docker compose run --rm anchor bash

deploy-devnet: ## deploy to Solana devnet (requires ~/.config/solana/id.json with SOL on devnet)
	docker compose run --rm \
		-v $$HOME/.config/solana:/root/.config/solana \
		anchor anchor deploy --provider.cluster devnet

idl: ## emit IDL JSON (after build)
	docker compose run --rm anchor anchor idl build

clean: ## remove target/ and volumes
	docker compose down -v
	rm -rf target/ .anchor/
