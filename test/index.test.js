const test = require('node:test')
const assert = require('node:assert/strict')

const danimoth = require('../index.js')
const packageJson = require('../package.json')

test('reports the same version as package.json', () => {
  assert.equal(danimoth.version(), packageJson.version)
})

test('throws from SDK functions until the SDK is loaded', () => {
  assert.throws(() => danimoth.sdkVersion(), /isn't loaded/)
  assert.throws(() => danimoth.runCallbacks(), /isn't loaded/)
  assert.throws(() => danimoth.createClient({}), /isn't loaded/)
})

test('throws when the SDK DLL is missing', () => {
  assert.throws(
    () => danimoth.loadSdk('C:\\does-not-exist\\discord_partner_sdk.dll'),
    /Failed to load the Discord Social SDK/,
  )
})
