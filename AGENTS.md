# AGENTS.md

Guidance for AI agents working in this repository.

- This is a Neon (Rust) Node.js native module, consumed by the ShieldBattery Electron app's main
  process. The Rust crate lives in `native/`; `index.js` loads the built `prebuilds/x64.node`.
- Never commit or publish the Discord Social SDK (`discord_partner_sdk.dll`, `discordpp.h`, or any
  other file from the SDK download). Keep a local copy under `sdk/`, which is gitignored and
  excluded from the package.
- Keep `version` in `native/Cargo.toml` equal to `version` in `package.json`.
- Run `pnpm test`, `pnpm run format:check`, `cargo fmt --all -- --check` and
  `cargo clippy --all-targets -- -D warnings` (the last two in `native/`) before committing; CI runs
  the same checks.
- Every export needs a matching declaration in `index.d.ts`.
