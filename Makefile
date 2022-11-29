ERROR=echo "\033[0;31m"
SUCCESS=echo "\033[0;32m"

CARGO_RUN_WATCH = RUST_LOG=debug cargo watch -x 'run --bin social-service -- --port 8080'
CARGO_RUN = RUST_LOG=debug cargo run -- --port 8080
RUN_LOCAL_DB = docker-compose up -d && docker exec social_service_db bash -c "until pg_isready; do sleep 1; done" && sleep 5
MIGRATE_UP = LOCAL=true RUST_LOG=debug MIGRATE=up cargo run -- --port 8080
MIGRATE_DOWN = LOCAL=true RUST_LOG=debug MIGRATE=down cargo run -- --port 8080

# Should run after migrations
GENERATE_MODELS = sea-orm-cli generate entity -u postgres://postgres:postgres@0.0.0.0:3500/social_service -o ./src/entities

WATCH_EXISTS = $(shell which cargo-watch > /dev/null && echo 1 || echo 0)
DOCKER_COMPOSE_EXISTS = $(shell which docker-compose > /dev/null && echo 1 || echo 0)
SEA_ORM_CLI_EXISTS = $(shell which sea-orm-cli > /dev/null && echo 1 || echo 0)

INSTALL_WATCH = cargo install cargo-watch
INSTALL_SEA_ORM_CLI = cargo install sea-orm-cli

DATE = $(shell date '+%Y%m%d')
REGEX = ".*_([0-9]{6})_.*"
FILE = $(shell ls -lt src/migrator | grep -E $(REGEX) | sed 's/.*_\([0-9]\{6\}\)_.*/\1/' | head -n 1)

dev:
ifeq ($(WATCH_EXISTS), 1)
	@make rundb
	@$(CARGO_RUN_WATCH)
else
	@$(ERROR) "cargo-watch not found. installing..."
	@$(INSTALL_WATCH)
	@$(CARGO_RUN_WATCH)
endif

run: 
	@make rundb
	@$(CARGO_RUN)

rundb:
ifeq ($(DOCKER_COMPOSE_EXISTS), 1)
	@$(RUN_LOCAL_DB)
else
	@$(ERROR) "Install Docker in order to run the local DB"
	@exit 1;
endif

query:
	@docker exec -it social_service_db bash -c "psql -U postgres -d social_service"

destroydb:
	@docker stop social_service_db
	@docker rm social_service_db
	@rm -rf ./postgres_data

migration:
ifdef name
	$(eval MIGRATION_DESC = "$(name)")
else
	@$(ERROR) "No name given for the migration file. Run the command with the 'name' argument\n" 
	@exit 1;
endif
ifeq ($(FILE),)
	$(eval FILE = $(shell echo "000000"))
endif
	$(eval PRE_NEW_INDEX := $(shell expr $(FILE) + 1))
	$(eval PRE_NEW_INDEX_LEN := $(shell printf '%s' '$(PRE_NEW_INDEX)' | wc -c ))
	$(eval REST := $(shell expr 6 - $(PRE_NEW_INDEX_LEN)))
	$(eval FILLER = $(shell printf '0%.0s' {1..$(REST)}))
	$(eval NEW_INDEX = $(shell echo $(FILLER)$(PRE_NEW_INDEX)))
	@chmod +x ./m.sh
	@$(SUCCESS) "Creating migration m$(DATE)_$(NEW_INDEX)_$(MIGRATION_DESC).rs"
	@./m.sh m$(DATE)_$(NEW_INDEX)_$(MIGRATION_DESC)

models:
ifeq ($(SEA_ORM_CLI_EXISTS), 1)
	@$(GENERATE_MODELS)
else
	@$(ERROR) "sea-orm-cli not found. installing..."
	@$(INSTALL_SEA_ORM_CLI)
	@$(GENERATE_MODELS)
endif

migrateup:
ifdef count
	$(eval COUNT = "$(count)")
else
	$(eval COUNT = "0")
endif
	@make rundb
	@$(SUCCESS) "Running and applying migrations"
	COUNT=$(COUNT) $(MIGRATE_UP)

migratedown:
ifdef count
	$(eval COUNT = "$(count)")
else
	$(eval COUNT = "0")
endif
	@make rundb
	@$(SUCCESS) "Rolling back migrations"
	COUNT=$(COUNT) $(MIGRATE_DOWN)
