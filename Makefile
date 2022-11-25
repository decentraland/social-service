ERROR=echo "\033[0;31m"
SUCCESS=echo "\033[0;32m"

CARGO_RUN_WATCH = RUST_LOG=debug cargo watch -x 'run --bin social-service -- --port 8080'
CARGO_RUN = RUST_LOG=debug cargo run -- --port 8080

WATCH_EXISTS = $(shell which cargo-watch > /dev/null && echo 1 || echo 0)
INSTALL_WATCH = cargo install cargo-watch
DATE = $(shell date '+%Y%m%d')
REGEX = ".*_([0-9]{6})_.*"
FILE = $(shell ls -lt src/migrator | grep -E $(REGEX) | sed 's/.*_\([0-9]\{6\}\)_.*/\1/' | head -n 1)

dev:
ifeq ($(WATCH_EXISTS), 1)
	@$(CARGO_RUN_WATCH)
else
	@echo "cargo-watch not found. installing..."
	@$(INSTALL_WATCH)
	@$(CARGO_RUN_WATCH)
endif

run: 
	@$(CARGO_RUN)

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