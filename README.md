# @shieldbattery/danimoth

Rust-based Node.js native module wrapping the Discord Social SDK.

## The Discord Social SDK is not included

Neither this repo nor the published package contains the Discord Social SDK
(`discord_partner_sdk.dll`, its headers, or anything else from the download). It comes from the
Downloads page of the Discord Developer Portal once the Social SDK is enabled for the application.
Keep a local copy under `sdk/`, which is gitignored and excluded from the package, by extracting
the download's `discord_social_sdk` folder there (so the DLL ends up at
`sdk/discord_social_sdk/bin/release/discord_partner_sdk.dll`).

The module builds without the SDK: it loads the DLL at runtime from a path the app passes to
`loadSdk`, so the app decides where the DLL lives and how it gets there. The C API declarations in
`native/src/sys.rs` are written by hand against SDK 1.10.

## Usage

```js
const discord = require('@shieldbattery/danimoth')

discord.loadSdk('C:\\path\\to\\discord_partner_sdk.dll')
// Client callbacks and promise completions only happen while runCallbacks runs.
setInterval(() => discord.runCallbacks(), 100)

const client = discord.createClient({
  onLog: (message, severity) => console.log(`[discord:${severity}] ${message}`),
  onStatusChanged: status => {
    if (status === 'ready') {
      discord.updateRichPresence(client, {
        details: 'Ranked 1v1',
        state: 'Terran vs Zerg',
        timestamps: { start: Date.now() },
      })
    }
  },
  onTokenExpiring: () => {
    // Get a new access token and pass it to updateToken.
  },
  onActivityJoin: joinSecret => {
    // Join whatever the secret identifies.
  },
})
await discord.updateToken(client, accessToken)
discord.connect(client)
```

See `index.d.ts` for the whole API.

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
The tests in `test/sdk.test.js` load the real SDK from the local copy under `sdk/` (or from the path
in `DANIMOTH_SDK_DLL`) and are skipped when it isn't there, as in CI. They don't connect to Discord.

## Releasing

Run the "Release package" workflow from the Actions tab with the new version number. It sets the
version in `package.json` and `native/Cargo.toml` (kept equal; the tests fail if they differ), runs
the tests, builds the release binary, publishes to npm, and pushes a "Version X" commit and a tag
with the bare version (`1.0.0`, not `v1.0.0`), matching the other ShieldBattery native modules.

The workflow publishes as an npm trusted publisher through GitHub's OIDC token, so no npm token is
stored in the repo or its secrets. The binary isn't code signed yet.

## License

MIT
