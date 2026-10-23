// Tests against the real SDK, which isn't in the repo: they run when the DLL is at
// DANIMOTH_SDK_DLL or in the local copy under sdk/, and are skipped otherwise (e.g. in CI). They
// don't connect to Discord.
const test = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const danimoth = require('../index.js')

const dllPath =
  process.env.DANIMOTH_SDK_DLL ??
  path.join(
    __dirname,
    '..',
    'sdk',
    'discord_social_sdk',
    'bin',
    'release',
    'discord_partner_sdk.dll',
  )
const skip = fs.existsSync(dllPath) ? false : `no SDK DLL at ${dllPath}`

/** Runs the SDK's callbacks until `promise` settles. */
async function pumpUntil(promise) {
  const pump = setInterval(() => danimoth.runCallbacks(), 10)
  try {
    return await promise
  } finally {
    clearInterval(pump)
  }
}

test('the real SDK', { skip }, async t => {
  danimoth.loadSdk(dllPath)

  await t.test('reports its version', () => {
    const version = danimoth.sdkVersion()
    assert.equal(version.major, 1)
    assert.match(version.hash, /^[0-9a-f]+$/)
  })

  await t.test('ignores loading it again', () => {
    danimoth.loadSdk('C:\\does-not-exist\\discord_partner_sdk.dll')
  })

  await t.test('creates a disconnected client', () => {
    const client = danimoth.createClient({})
    try {
      assert.equal(danimoth.getStatus(client), 'disconnected')
    } finally {
      danimoth.destroyClient(client)
    }
  })

  await t.test('rejects rich presence without an application ID, and logs why', async () => {
    const logs = []
    const client = danimoth.createClient({
      onLog: (message, severity) => logs.push({ message, severity }),
    })
    try {
      const error = await pumpUntil(
        danimoth.updateRichPresence(client, { details: 'Testing' }).then(
          () => assert.fail('expected a rejection'),
          e => e,
        ),
      )
      assert.equal(error.errorType, 'validationError')
      assert.match(error.message, /application ID/i)
      assert.equal(error.retryable, false)
      // Log messages are delivered through the event loop after the call that received them.
      await new Promise(resolve => setImmediate(resolve))
      assert.ok(
        logs.some(l => l.severity === 'error' && /application ID/i.test(l.message)),
        JSON.stringify(logs),
      )
    } finally {
      danimoth.destroyClient(client)
    }
  })

  await t.test('throws on activity fields it can not convert', () => {
    const client = danimoth.createClient({})
    try {
      assert.throws(() => danimoth.updateRichPresence(client, { type: 'sleeping' }), TypeError)
      assert.throws(
        () =>
          danimoth.updateRichPresence(client, {
            party: { id: 'abc', currentSize: 1.5, maxSize: 8 },
          }),
        RangeError,
      )
      assert.throws(
        () => danimoth.updateRichPresence(client, { timestamps: { start: -1 } }),
        RangeError,
      )
      assert.throws(
        () => danimoth.updateRichPresence(client, { supportedPlatforms: ['dreamcast'] }),
        TypeError,
      )
      assert.throws(() => danimoth.setApplicationId(client, 'not-a-snowflake'), TypeError)
    } finally {
      danimoth.destroyClient(client)
    }
  })

  await t.test('throws from calls with a destroyed client', () => {
    const client = danimoth.createClient({})
    danimoth.destroyClient(client)
    assert.throws(() => danimoth.getStatus(client), /destroyed/)
    assert.throws(() => danimoth.connect(client), /destroyed/)
    // Destroying twice is harmless.
    danimoth.destroyClient(client)
  })
})
