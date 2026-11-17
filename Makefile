# vuretro — common commands. Run `make` to list them.
SHELL := /bin/bash
UNAME := $(shell uname -s)

ifeq ($(UNAME),Darwin)
  LIB_EXT := dylib
  LIB_PREFIX := lib
else ifeq ($(findstring MINGW,$(UNAME))$(findstring MSYS,$(UNAME)),)
  LIB_EXT := so
  LIB_PREFIX := lib
else
  LIB_EXT := dll
  LIB_PREFIX :=
endif

TEST_CORE := target/debug/$(LIB_PREFIX)vuretro_test_core.$(LIB_EXT)
TEST_ROM  := target/game.test
FBNEO     := cores/bin/fbneo_libretro.$(LIB_EXT)
PCSX      := cores/bin/pcsx_rearmed_libretro.$(LIB_EXT)

# Override on the command line: make play ROM=~/Games/mslug.zip CORE=fbneo SCALE=4
ROM   ?=
CORE  ?= fbneo
SCALE ?= 2
ROMS  ?= $(HOME)/Games/roms

.DEFAULT_GOAL := help
.PHONY: help dev run play build release test fmt lint check ci cores scan screenshot clean

help: ## List available commands
	@awk 'BEGIN{FS=":.*## "} /^[a-z-]+:.*## /{printf "  \033[33mmake %-12s\033[0m %s\n",$$1,$$2}' $(MAKEFILE_LIST)

dev: $(TEST_ROM) ## Build and run the runner with the fake test core (no ROM/BIOS needed)
	cargo build -p vuretro-test-core
	cargo run -p vuretro-runner -- --core $(TEST_CORE) --rom $(TEST_ROM) --scale $(SCALE)

$(TEST_ROM):
	@mkdir -p $(dir $@) && echo test > $@

play: ## Play a real game: make play ROM=~/Games/mslug.zip [CORE=fbneo|pcsx] [SCALE=4]
	@test -n "$(ROM)" || { echo "missing ROM=<path>"; exit 1; }
	@core="$(FBNEO)"; [ "$(CORE)" = "pcsx" ] && core="$(PCSX)"; \
	  test -f "$$core" || { echo "$$core not found, run: make cores"; exit 1; }; \
	  cargo run --release -p vuretro-runner -- --core "$$core" --rom "$(ROM)" --scale $(SCALE)

run: play ## Alias for play

build: ## Debug build of the whole workspace
	cargo build --workspace

release: ## Release build of the whole workspace
	cargo build --workspace --release

test: ## Run unit tests
	cargo test --all

fmt: ## Format code
	cargo fmt --all

lint: ## fmt --check + clippy (same as CI)
	cargo fmt --all --check
	cargo clippy --all-targets -- -D warnings

check: lint test ## lint + test

ci: check screenshot ## Run every step GitHub Actions runs

cores: ## Download fbneo + pcsx_rearmed cores into cores/bin/
	./cores/fetch-cores.sh

scan: ## Scan a ROM folder: make scan ROMS=~/Games/roms
	cargo run -p vuretro-library --bin vuretro-scan -- "$(ROMS)"

screenshot: $(TEST_ROM) ## Run 120 headless frames, write target/shot.ppm (CI smoke test)
	cargo build -p vuretro-test-core
	cargo run -p vuretro-runner -- --core $(TEST_CORE) --rom $(TEST_ROM) \
	  --headless-frames 120 --screenshot target/shot.ppm \
	  --system-dir target/tmp/system --save-dir target/tmp/saves
	@test -s target/shot.ppm && echo "OK → target/shot.ppm"

clean: ## Remove target/
	cargo clean
