import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const directory = new URL('../.github/workflows/', import.meta.url);
const workflows = readdirSync(directory)
  .filter((name) => /\.ya?ml$/.test(name))
  .map((name) => [name, readFileSync(new URL(name, directory), 'utf8')]);

test('teLLeM-006 external actions use commit pins', () => {
  assert.ok(workflows.length > 0, 'expected at least one workflow');
  for (const [name, source] of workflows) {
    for (const match of source.matchAll(/^\s*-?\s*uses:\s*(\S+)(.*)$/gm)) {
      const [, action, comment] = match;
      if (action.startsWith('./')) continue;
      assert.match(action, /^[^@]+@[0-9a-f]{40}$/, `${name}: ${action}`);
      assert.match(comment, /^\s+#\s*\S+/, `${name}: missing ref comment`);
      if (action.startsWith('dtolnay/rust-toolchain@')) {
        const step = source.slice(match.index).split(/\n\s*- /)[0];
        assert.match(step, /\n\s+with:\s*\n(?:[ \t]+[^\n]*\n)*?[ \t]+toolchain: stable\s*$/m);
      }
    }
  }
});
