# @shieldbattery/danimoth

Rust-based Node.js native module wrapping the Discord Social SDK.

## The Discord Social SDK is not included

Neither this repo nor the published package contains the Discord Social SDK
(`discord_partner_sdk.dll`, its headers, or anything else from the download). It comes from the
Downloads page of the Discord Developer Portal once the Social SDK is enabled for the application.
Keep a local copy under `sdk/`, which is gitignored and excluded from the package.

## OS support

Currently we only build for x64 Windows.

## Local development

Build the module and copy it to where `index.js` loads it from:

```sh
pnpm install
pnpm run build-debug
pnpm run tag-prebuild
```

To consume your local copy from the ShieldBattery app, point the dependency in
`ShieldBattery/app/package.json` at this directory with pnpm's `link:` protocol, then install. The
path is relative to the `app` directory; with the two checkouts side by side it is:

```json
"@shieldbattery/danimoth": "link:../../danimoth"
```

```sh
cd path/to/ShieldBattery/app
pnpm install
```

After rebuilding here, run `build-debug` and `tag-prebuild` again, then rebuild the app. It picks up
the new `prebuilds/x64.node` through the link without reinstalling.

## Tests

```sh
pnpm test
```

This runs the Rust tests, builds the module, and runs the Node tests in `test/` against the build.

## Versioning

Releases are tagged with the bare version (`1.0.0`, not `v1.0.0`), matching the other ShieldBattery
native modules. The version in `native/Cargo.toml` stays equal to the one in `package.json`, and
the tests fail if they differ. The release workflow (publishing and code signing) isn't set up yet.

## License

MIT
