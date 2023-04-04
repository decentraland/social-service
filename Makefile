ERROR=echo "\033[0;31m"
SUCCESS=echo "\033[0;32m"

CARGO_RUN_WATCH = RUST_LOG=debug cargo watch -x 'run --bin social-service -- --port 8080'
CARGO_RUN = RUST_LOG=debug cargo run -- --port 8080
RUN_LOCAL_DB = docker-compose up -d && docker exec social_service_db bash -c "until pg_isready; do sleep 1; done" && sleep 5
LOCAL_DB = $(shell docker ps | grep social_service_db > /dev/null && echo 1 || echo 0)

WATCH_EXISTS = $(shell which cargo-watch > /dev/null && echo 1 || echo 0)
DOCKER_COMPOSE_EXISTS = $(shell which docker-compose > /dev/null && echo 1 || echo 0)
SQLX_EXISTS = $(shell which sqlx > /dev/null && echo 1 || echo 0)

INSTALL_WATCH = cargo install cargo-watch
INSTALL_SQLX = cargo install sqlx-cli

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
ifeq ($(SQLX_EXISTS), 1)
	@sqlx migrate add -r $(name)
else
	@$(ERROR) "sqlx-cli is not installed. Installing..."
	@$(INSTALL_SQLX)
	@sqlx migrate add -r $(name)
endif


# it should be used locally
test:
ifeq ($(LOCAL_DB), 1)
	@docker stop social_service_db 2>/dev/null
	@mv ./postgres_data ./postgres_data_2 2>/dev/null
	@$(RUN_LOCAL_DB)
	-@cargo test
	@docker stop social_service_db
	@rm -rf ./postgres_data
	@mv ./postgres_data_2 ./postgres_data
else
	@$(RUN_LOCAL_DB)
	-@cargo test
	@make destroydb
endif

test-d:
ifeq ($(LOCAL_DB), 1)
	@docker stop social_service_db 2>/dev/null
	@mv ./postgres_data ./postgres_data_2 2>/dev/null
	@$(RUN_LOCAL_DB)
	-RUST_LOG=debug cargo test
	@docker stop social_service_db
	@rm -rf ./postgres_data
	@mv ./postgres_data_2 ./postgres_data
else
	@$(RUN_LOCAL_DB)
	-RUST_LOG=debug cargo test
	@make destroydb
endif
