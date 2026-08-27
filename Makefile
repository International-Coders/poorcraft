# LOREFORGE (POORCRAFT) — make targets
# Living documentation of what can be done and how. Agents: keep this file
# in sync whenever commands change, and log each job in DEVLOG.md.

.PHONY: help build test run smoke vistest package runtimes push

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

smoke: ## Launch the game in the background for ~12s and verify it stays up
	@./target/release/loreforge & sleep 12; \
	if pgrep -f target/release/loreforge > /dev/null; then echo "SMOKE OK"; else echo "SMOKE FAIL"; exit 1; fi; \
	pkill -f target/release/loreforge || true

perf: ## Frame-time benchmark (p50/p95) of a representative scene
	cargo run --release -p xtask -- perf terrain_vista 30

vistest: ## Render every proof scene into shots/
	cargo run --release -p xtask -- vistest shots

screenshot: ## Render one scene: make screenshot SCENE=terrain_vista OUT=shots/x.png
	cargo run --release -p xtask -- screenshot $(SCENE) $(OUT)

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

## Scaffold a new mod folder (Step 39): make new-mod id=foo name="Foo"
new-mod:
	cargo run -p xtask -- new-mod $(id) $(if $(name),--name "$(name)",)
