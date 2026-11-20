# vuretro — common commands. Run `make` to list them.
SHELL := /bin/bash
# rustup installs cargo here; make it work even in terminals opened before the install
export PATH := $(HOME)/.cargo/bin:$(PATH)
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
SNES9X    := cores/bin/snes9x_libretro.$(LIB_EXT)
MGBA      := cores/bin/mgba_libretro.$(LIB_EXT)

# Override on the command line: make play ROM=~/Games/mslug.zip CORE=fbneo SCALE=4  (CORE: fbneo|pcsx|snes|gba)
ROM   ?=
CORE  ?= fbneo
SCALE ?= 2
ROMS  ?= $(HOME)/Games/roms

.DEFAULT_GOAL := help
.PHONY: help setup toolchain dev demo app-build run play options build release test fmt lint check ci cores scan screenshot smoke clean

help: ## List available commands
	@awk 'BEGIN{FS=":.*## "} /^[a-z-]+:.*## /{printf "  \033[33mmake %-12s\033[0m %s\n",$$1,$$2}' $(MAKEFILE_LIST)

setup: ## Install prerequisites on a fresh machine (Rust, checks Node/Xcode CLT)
	./scripts/setup.sh

# Auto-runs setup the first time cargo is missing
toolchain:
	@command -v cargo >/dev/null || ./scripts/setup.sh

# `tauri dev` only watches src-tauri; also watch crates/ so edits to the shared crates
# (settings, controls, library) rebuild the app instead of leaving a stale backend.
# Recipes below go through the shell (`export PATH` + `;`): make 3.81 execs simple
# commands directly using the *parent* PATH, so a cargo installed during this very
# run would not be found.
dev: toolchain node_modules ## Open the vuretro app (Library UI); builds the runner and fake core first
	@export PATH="$$HOME/.cargo/bin:$$PATH"; set -e; \
	  cargo build -p vuretro-runner -p vuretro-test-core; \
	  npx tauri dev --additional-watch-folders crates

app-build: toolchain node_modules ## Build the installable app bundle (release, runner bundled inside)
	@export PATH="$$HOME/.cargo/bin:$$PATH"; set -e; \
	  ./scripts/prepare-sidecar.sh; \
	  npx tauri build --config src-tauri/tauri.bundle.conf.json

node_modules: package.json
	npm install

demo: $(TEST_ROM) ## Run the runner alone with the fake test core (no UI, no ROM needed)
	cargo build -p vuretro-test-core
	cargo run -p vuretro-runner -- --core $(TEST_CORE) --rom $(TEST_ROM) --scale $(SCALE)

$(TEST_ROM):
	@mkdir -p $(dir $@) && echo test > $@

play: ## Play a real game: make play ROM=<file> [CORE=fbneo|pcsx|snes|gba] [SCALE=4]
	@test -n "$(ROM)" || { echo "missing ROM=<path>"; exit 1; }
	@core="$(FBNEO)"; \
	  case "$(CORE)" in pcsx) core="$(PCSX)";; snes) core="$(SNES9X)";; gba) core="$(MGBA)";; fbneo) ;; *) echo "unknown CORE=$(CORE) (fbneo|pcsx|snes|gba)"; exit 1;; esac; \
	  test -f "$$core" || { echo "$$core not found, run: make cores"; exit 1; }; \
	  cargo run --release -p vuretro-runner -- --core "$$core" --rom "$(ROM)" --scale $(SCALE)

run: play ## Alias for play

options: ## Print a core options template: make options [CORE=snes] ROM=<file>
	@core="$(FBNEO)"; \
	  case "$(CORE)" in pcsx) core="$(PCSX)";; snes) core="$(SNES9X)";; gba) core="$(MGBA)";; esac; \
	  cargo run -q -p vuretro-runner -- --core "$$core" --rom "$(ROM)" --list-options

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

ci: check smoke ## Run every step GitHub Actions runs

cores: ## Download fbneo, pcsx_rearmed, snes9x, mgba cores into cores/bin/
	./cores/fetch-cores.sh

cores-offline: ## Download core zips into cores/offline/ so they can be installed with no internet (ARGS=--all for the alternatives too)
	./cores/fetch-offline.sh $(ARGS)

scan: ## Scan a ROM folder: make scan ROMS=~/Games/roms
	cargo run -p vuretro-library --bin vuretro-scan -- "$(ROMS)"

smoke: ## Smoke test: video, save state, SRAM, core options (fake core)
	cargo build -p vuretro-test-core
	./scripts/smoke.sh $(TEST_CORE)

screenshot: $(TEST_ROM) ## Run 120 headless frames, write target/shot.ppm (CI smoke test)
	cargo build -p vuretro-test-core
	cargo run -p vuretro-runner -- --core $(TEST_CORE) --rom $(TEST_ROM) \
	  --headless-frames 120 --screenshot target/shot.ppm \
	  --system-dir target/tmp/system --save-dir target/tmp/saves
	@test -s target/shot.ppm && echo "OK → target/shot.ppm"

clean: ## Remove target/
	cargo clean
