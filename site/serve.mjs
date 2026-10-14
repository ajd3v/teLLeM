// Static file server for the teLLeM site. No dependencies, matching the rest of
// the stack. HTML is no-cache so a rebuild's ?v= stamp always reaches the
// browser; everything else is content-addressed by that stamp and cached hard.
import { createServer } from 'node:http';
import { readFileSync, realpathSync, statSync } from 'node:fs';
import { extname, join, relative, isAbsolute, sep } from 'node:path';

const ROOT = realpathSync(process.env.SITE_ROOT ?? new URL('.', import.meta.url).pathname);
const MIME = {
  '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript',
  '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.woff2': 'font/woff2',
  '.json': 'application/json', '.toml': 'text/plain; charset=utf-8', '.svg': 'image/svg+xml', '.ico': 'image/x-icon',
};

createServer((req, res) => {
  let path;
  try {
    path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  } catch {
    res.writeHead(400, { 'Content-Type': 'text/plain' });
    return res.end('400');
  }
  // canonical home moved to tellem.alanj.dev; the old host 301s there
  const host = String(req.headers.host || '').split(':')[0];
  if (host === 'tellem.gainful.work') {
    res.writeHead(301, { Location: 'https://tellem.alanj.dev' + req.url });
    return res.end();
  }
  if (path === '/healthz') {
    res.writeHead(200, { 'Content-Type': 'text/plain', 'Cache-Control': 'no-store' });
    return res.end('ok');
  }
  if (path.endsWith('/')) path += 'index.html';
  let file, body;
  try {
    file = realpathSync(join(ROOT, path));
    const inside = relative(ROOT, file);
    if (inside === '..' || inside.startsWith('..' + sep) || isAbsolute(inside) || !statSync(file).isFile()) {
      throw new Error('outside site or not a file');
    }
    body = readFileSync(file);
  } catch {
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    return res.end('404');
  }
  const ext = extname(file);
  res.writeHead(200, {
    'Content-Type': MIME[ext] ?? 'application/octet-stream',
    'Cache-Control': ext === '.html' ? 'no-cache' : 'public, max-age=604800, immutable',
    'X-Content-Type-Options': 'nosniff',
    'Referrer-Policy': 'strict-origin-when-cross-origin',
  });
  res.end(body);
}).listen(Number(process.env.PORT ?? 8940), process.env.BIND ?? '0.0.0.0', () => {
  console.log(`tellem site on ${process.env.BIND ?? '0.0.0.0'}:${process.env.PORT ?? 8940}`);
});
