// Node-side parity check for the napi binding. Run: node parity.test.mjs
// (CI builds the cdylib and copies it to tellem.node first.)
import { createRequire } from 'node:module';
import assert from 'node:assert/strict';

const { deAi, lintJson, setPack } = createRequire(import.meta.url)('./tellem.node');

// Extra packs load before the first call. The rules themselves are covered by
// tests/gainful_parity.rs, this only proves the binding carries a pack through.
setPack(`
[meta]
name = "test-voice"
version = "0.1.0"
[[rules]]
id = "G001"
name = "roles-to-jobs"
kind = "word"
rationale = "job listings are jobs"
[rules.swaps]
roles = "jobs"
`);
assert.equal(deAi('Python roles'), 'Python jobs');
assert.throws(() => setPack('[meta]\nname="x"\nversion="1"\n'), /once/);

assert.equal(deAi('I build software — and I ship it.'), 'I build software, and I ship it.');
assert.equal(deAi('2007—2024'), '2007 to 2024'); // deviation: never a comma
assert.equal(deAi('the eval harness runs nightly'), 'the eval harness runs nightly');
assert.equal(deAi('Architected a resilient framework'), 'Built a resilient framework');
assert.equal(deAi("It's worth noting that we shipped it."), 'We shipped it.');

const report = JSON.parse(lintJson('We delve into a rich tapestry.'));
assert.ok(report.findings.some((f) => f.rule_id === 'T013'));
assert.ok(report.findings.every((f) => f.rationale.length > 0));

console.log('node parity: ok');
