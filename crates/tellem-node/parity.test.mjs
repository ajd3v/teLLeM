// Node-side parity check for the napi binding. Run: node parity.test.mjs
// (CI builds the cdylib and copies it to tellem.node first.)
import { createRequire } from 'node:module';
import assert from 'node:assert/strict';

const { deAi, lintJson } = createRequire(import.meta.url)('./tellem.node');

assert.equal(deAi('I build software — and I ship it.'), 'I build software, and I ship it.');
assert.equal(deAi('2007—2024'), '2007 to 2024'); // deviation: never a comma
assert.equal(deAi('the eval harness runs nightly'), 'the eval harness runs nightly');
assert.equal(deAi('Architected a resilient framework'), 'Built a resilient framework');
assert.equal(deAi("It's worth noting that we shipped it."), 'We shipped it.');

const report = JSON.parse(lintJson('We delve into a rich tapestry.'));
assert.ok(report.findings.some((f) => f.rule_id === 'T013'));
assert.ok(report.findings.every((f) => f.rationale.length > 0));

console.log('node parity: ok');
