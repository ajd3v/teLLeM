import assert from 'node:assert/strict';
import { test } from 'node:test';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { tmpdir } from 'node:os';
import { runInNewContext } from 'node:vm';

const source = fs.readFileSync(new URL('./serve.mjs', import.meta.url), 'utf8')
  .replace(/^import .*;\n/gm, '')
  .replace("new URL('.', import.meta.url).pathname", "''");

function server(root) {
  let handler;
  runInNewContext(source, {
    ...fs, ...path, URL,
    process: { env: { SITE_ROOT: root } },
    createServer(callback) {
      handler = callback;
      return { listen() {} };
    },
  });
  return (url, host = 'localhost') => {
    const response = {};
    handler({ url, headers: { host } }, {
      writeHead(status, headers) { Object.assign(response, { status, headers }); },
      end(body) { response.body = body; },
    });
    return response;
  };
}

test('malformed targets return 400 and subsequent requests still work', () => {
  const request = server('/unused');
  for (const target of ['/%E0%A4', '//', '/%zz']) {
    for (const host of ['localhost', 'tellem.gainful.work']) {
      assert.equal(request(target, host).status, 400);
      assert.equal(request('/healthz').body, 'ok');
    }
  }
});

test('home page and canonical redirect still work', (t) => {
  const root = fs.mkdtempSync(path.join(tmpdir(), 'tellem-site-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.writeFileSync(path.join(root, 'index.html'), '<h1>synthetic</h1>');
  const request = server(root);
  assert.equal(request('/').status, 200);
  assert.equal(request('/').body.toString(), '<h1>synthetic</h1>');
  assert.equal(request('/missing').status, 404);
  const redirect = request('/?v=1', 'tellem.gainful.work');
  assert.equal(redirect.status, 301);
  assert.equal(redirect.headers.Location, 'https://tellem.alanj.dev/?v=1');
});
