# LOREFORGE (POORCRAFT) — make targets
# Living documentation of what can be done and how. Agents: keep this file
# in sync whenever commands change, and log each job in DEVLOG.md.

.PHONY: help build test run smoke vistest perf package runtimes push night-plan-check seedlab sounds

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Debug build of the whole workspace
	cargo build --workspace

release: ## Release build (game + server)
	cargo build --release -p loreforge -p loreforge-server

test: ## Run the full test suite
	cargo test --workspace

run: ## Play the game (title screen)
	cargo run --release -p loreforge

server: ## Run the dedicated multiplayer server
	cargo run --release -p loreforge-server

smoke: release ## Headless logic smoke (300 ticks: worldgen, mob AI, NPC schedule, craft, mine) + 12s GUI liveness
	@./target/release/loreforge --smoke > smoke_run.log 2>&1; \
	code=$$?; \
	if [ $$code -ne 0 ]; then echo "SMOKE FAIL (logic exit $$code)"; cat smoke_run.log; exit 1; fi; \
	if grep -qE "(PANIC|thread.*panicked|ERROR.*wgpu|vulkan.*error)" smoke_run.log; then \
		echo "SMOKE FAIL (error pattern in log)"; grep -E "(PANIC|thread.*panicked|ERROR.*wgpu|vulkan.*error)" smoke_run.log; exit 1; fi; \
	echo "smoke (headless logic): OK"; \
	./target/release/loreforge > /dev/null 2>&1 & sleep 12; \
	if pgrep -f target/release/loreforge > /dev/null; then echo "SMOKE OK"; else echo "SMOKE FAIL (gui)"; exit 1; fi; \
	pkill -f target/release/loreforge || true

perf: ## Frame-time benchmark (p50/p95) of a representative scene
	cargo run --release -p xtask -- perf terrain_vista 30

vistest: ## Render every proof scene into shots/
	cargo run --release -p xtask -- vistest shots

screenshot: ## Render one scene: make screenshot SCENE=terrain_vista OUT=shots/x.png
	cargo run --release -p xtask -- screenshot $(SCENE) $(OUT)

sounds: ## Generate missing sound effects via ElevenLabs (needs ELEVENLABS_API_KEY; cached files are kept)
	@if [ -z "$$ELEVENLABS_API_KEY" ]; then echo "set ELEVENLABS_API_KEY first"; exit 2; fi
	python3 tools/gen_sounds.py

package: ## Portable zip distribution into dist/
	cargo run --release -p xtask -- package

runtimes: release ## macOS .app + .dmg + Linux tarball (+ Windows exe if mingw present) into dist/
	@mkdir -p dist/loreforge.app/Contents/MacOS dist/loreforge.app/Contents/Resources
	@cp target/release/loreforge dist/loreforge.app/Contents/MacOS/
	@cp target/release/loreforge-server dist/
	@printf 'APPLLORE' > dist/loreforge.app/Contents/Resources/PkgInfo
	@printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict><key>CFBundleExecutable</key><string>loreforge</string><key>CFBundleIdentifier</key><string>com.loreforge.game</string><key>CFBundleName</key><string>LOREFORGE</string><key>NSHighResolutionCapable</key><true/></dict></plist>\n' > dist/loreforge.app/Contents/Info.plist
	hdiutil create -volname LOREFORGE -srcfolder dist/loreforge.app -ov -format UDZO dist/loreforge-macos.dmg
	tar -czf dist/loreforge-linux-x86_64.tar.gz -C target/release loreforge loreforge-server
	@if command -v x86_64-w64-mingw32-gcc > /dev/null 2>&1; then \
		cargo build --release -p loreforge --target x86_64-pc-windows-gnu && \
		cp target/x86_64-pc-windows-gnu/release/loreforge.exe dist/; \
	else echo "NOTE: mingw not installed — skipping Windows exe (see AGENTS.md)"; fi
	@ls -la dist/

push: ## Commit-and-push helper: pushes current branch to the GitHub remote
	git push -u github HEAD || (git remote add github https://github.com/International-Coders/poorcraft.git && git push -u github HEAD)

night-plan-check: ## Validate the ZCode nightly alpha-to-beta goal pack
	cargo run -p xtask -- night-plan-check

seedlab: ## 64-seed diversity report -> target/seedlab_report.json (N05)
	cargo run --release -p xtask -- seedlab

truth: ## Runtime truth dashboard -> target/truth_report.json (B01); bench: make truth BENCH=terrain_vista
	cargo run --release -p xtask -- truth $(if $(BENCH),--bench $(BENCH) 120,)

p3d-build: ## Build the POORCRAFT 3D workspace (separate greenfield project)
	cargo build --release --manifest-path poorcraft3d/Cargo.toml

p3d-test: ## Run the POORCRAFT 3D test suite
	cargo test --manifest-path poorcraft3d/Cargo.toml

p3d-smoke: ## Headless liveness smoke for POORCRAFT 3D (runs the empty-world runtime 5 s)
	cargo build --release --manifest-path poorcraft3d/Cargo.toml
	poorcraft3d_bin=$$(pwd)/poorcraft3d/target/release/poorcraft3d; \
	$$poorcraft3d_bin --run 5 || exit 1; \
	echo "P3D SMOKE OK"

## Scaffold a new mod folder (Step 39): make new-mod id=foo name="Foo"
new-mod:
	cargo run -p xtask -- new-mod $(id) $(if $(name),--name "$(name)",)
