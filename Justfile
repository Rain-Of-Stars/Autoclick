set shell := ["pwsh", "-NoLogo", "-Command"]

fmt:
  cargo fmt --all

check:
  cargo check --workspace

lint:
  cargo clippy --workspace --all-targets -- -D warnings
  pnpm typecheck

test:
  cargo test --workspace
  pnpm test

bench:
  cargo bench --workspace

dev:
  pnpm tauri:dev

build:
  pnpm build
  pnpm tauri:build

release-check:
  ./scripts/release_check.ps1
