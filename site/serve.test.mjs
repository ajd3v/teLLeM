import assert from 'node:assert/strict';
import { test } from 'node:test';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { tmpdir } from 'node:os';
import { runInNewContext } from 'node:vm';
import { createHash } from 'node:crypto';

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
    const response = { headers: {} };
    handler({ url, headers: { host } }, {
      setHeader(name, value) { response.headers[name] = value; },
      writeHead(status, headers) {
        response.status = status;
        Object.assign(response.headers, headers);
      },
      end(body) { response.body = body; },
    });
    return response;
  };
}

test('malformed targets return 400 and subsequent requests still work', () => {
  const request = server(tmpdir());
  for (const target of ['/%E0%A4', '//', '/%zz']) {
    for (const host of ['localhost', 'tellem.gainful.work']) {
      assert.equal(request(target, host).status, 400);
      assert.equal(request('/healthz').body, 'ok');
    }
  }
});

test('security headers permit the shipped scripts and block framing', () => {
  const request = server(new URL('.', import.meta.url).pathname);
  const html = fs.readFileSync(new URL('./index.html', import.meta.url), 'utf8');
  const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)];
  assert.ok(scripts.length > 0);
  for (const url of ['/', '/healthz', '/missing', '/%zz']) {
    const { headers } = request(url);
    const csp = headers['Content-Security-Policy'];
    assert.ok(csp.includes("default-src 'self'"));
    assert.ok(csp.includes("script-src 'self' 'wasm-unsafe-eval'"));
    assert.ok(csp.includes("frame-ancestors 'none'"));
    assert.equal(headers['X-Frame-Options'], 'DENY');
    assert.equal(headers['Strict-Transport-Security'], 'max-age=31536000');
    for (const [, script] of scripts) {
      const hash = createHash('sha256').update(script).digest('base64');
      assert.ok(csp.includes("'sha256-" + hash + "'"), 'inline script hash missing');
    }
  }
});

test('paths and symlinks cannot escape the site root', (t) => {
  const dir = fs.mkdtempSync(path.join(tmpdir(), 'tellem-path-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const root = path.join(dir, 'site');
  fs.mkdirSync(root);
  fs.mkdirSync(root + '-private');
  fs.writeFileSync(path.join(root + '-private', 'marker.js'), 'synthetic');
  fs.symlinkSync(root + '-private', path.join(root, 'tellem'));
  fs.writeFileSync(path.join(root, 'index.html'), 'home');
  fs.symlinkSync(path.join(root, 'index.html'), path.join(root, 'styles.css'));
  for (const configuredRoot of [root, root + path.sep]) {
    const request = server(configuredRoot);
    assert.equal(request('/..%2fsite-private%2fmarker.js').status, 404);
    assert.equal(request('/tellem/marker.js').status, 404);
    assert.equal(request('/styles.css').body.toString(), 'home');
    assert.equal(request('/%00').status, 404);
  }
});

test('only public site assets can be served', (t) => {
  const root = fs.mkdtempSync(path.join(tmpdir(), 'tellem-assets-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const publicFiles = ['index.html', 'styles.css', 'fonts/fonts.css', 'fonts/font.woff2',
    'tellem/bench.js', 'tellem/tellem_wasm.js', 'tellem/tellem_wasm_bg.wasm',
    'tellem/catalog.toml', 'tellem/atlas.json'];
  const privateFiles = ['README.md', 'serve.mjs', 'atlas.py', 'stamp.py',
    'deploy/Dockerfile', 'deploy/push.sh', 'deploy/run-on-box.sh'];
  for (const file of [...publicFiles, ...privateFiles]) {
    fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    fs.writeFileSync(path.join(root, file), 'synthetic');
  }
  const request = server(root);
  for (const file of publicFiles) assert.equal(request('/' + file + '?v=abc').status, 200, file);
  for (const file of privateFiles) assert.equal(request('/' + file).status, 404, file);
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
