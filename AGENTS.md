# AGENTS.md

Guidance for AI agents working in this repository.

- This is a Neon (Rust) Node.js native module, consumed by the ShieldBattery Electron app's main
  process. The Rust crate lives in `native/`; `index.js` loads the built `prebuilds/x64.node`.
- Never commit or publish the Discord Social SDK (`discord_partner_sdk.dll`, `discordpp.h`, or any
  other file from the SDK download). Keep a local copy under `sdk/`, which is gitignored and
  excluded from the package.
- Never commit secrets: OAuth client secrets, access or refresh tokens, `.env` files. The repo is
  public. Tests never need a token, and tokens for manual testing stay outside the checkout.
- Keep the module free of any one application's details. Application IDs, launch commands and
  scheme names are passed in by the app using it, never hardcoded here.
- The SDK's C API is declared by hand in `native/src/sys.rs` and the DLL is loaded at runtime, so the
  module builds without the SDK. Every declaration must match `cdiscord.h` from the local SDK copy
  exactly; when moving to a new SDK version, check each one against the new header.
- SDK callbacks reach JS through a Neon `Channel`, never by calling JS directly, so it doesn't
  matter which thread the SDK calls them on. A callback that hands over a `Discord_ClientResult*`
  or a `Discord_String` gives ownership to the callee: drop the result and `Discord_Free` the
  string (see `discordpp.h`, which shows the ownership rules for every function).
- Keep `version` in `native/Cargo.toml` equal to `version` in `package.json`.
- Run `pnpm test`, `pnpm run format:check`, `cargo fmt --all -- --check` and
  `cargo clippy --all-targets -- -D warnings` (the last two in `native/`) before committing; CI runs
  the same checks.
- Every export needs a matching declaration in `index.d.ts`.
