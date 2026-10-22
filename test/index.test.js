const test = require('node:test')
const assert = require('node:assert/strict')

const { version } = require('../index.js')
const packageJson = require('../package.json')

test('reports the same version as package.json', () => {
  assert.equal(version(), packageJson.version)
})
