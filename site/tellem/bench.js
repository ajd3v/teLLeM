/* The bench. Everything here runs in the visitor's tab: the engine is Rust
   compiled to wasm and the text never leaves the page.

   Two payloads load lazily, on the first click that needs them, because
   together they are about 4MB raw and the page should not pay for that up
   front. The catalog only loads when someone asks `who`. */

const SAMPLE =
  "It's worth noting that our meticulous approach leverages cutting-edge tools to " +
  'showcase a myriad of possibilities. In today’s fast-paced world, this ' +
  'transformative platform boasts seamless integration — a testament to synergy. ' +
  'Needless to say, we delve into the ever-evolving landscape of software.';

const V = new URL(import.meta.url).search;
const $ = (id) => document.getElementById(id);
const out = $('out');
const status = $('status');
const input = $('input');
const buttons = [...document.querySelectorAll('.bench-actions button')];

let engine;
let catalogLoaded = false;

async function boot() {
  if (engine) return engine;
  status.textContent = 'loading engine';
  const mod = await import(`/tellem/tellem_wasm.js${V}`);
  await mod.default({ module_or_path: `/tellem/tellem_wasm_bg.wasm${V}` });
  engine = mod;
  return engine;
}

async function bootCatalog() {
  if (catalogLoaded) return;
  status.textContent = 'loading catalog';
  const res = await fetch(`/tellem/catalog.toml${V}`);
  if (!res.ok) throw new Error(`catalog ${res.status}`);
  (await boot()).load_catalog(await res.text());
  catalogLoaded = true;
}

function el(tag, cls, text) {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text !== undefined) n.textContent = text;
  return n;
}

function renderLint(report) {
  out.replaceChildren();
  if (!report.findings.length) {
    out.append(el('p', 'empty', 'No tells fired. That reads human, which is not the same as proof that it is.'));
  }
  for (const f of report.findings) {
    const row = el('div', 'finding');
    row.append(el('span', 'f-id', f.rule_id));
    row.append(el('span', null, f.rule_name));
    row.append(el('span', 'f-hit', f.excerpt));
    row.append(el('p', 'f-why', f.rationale));
    out.append(row);
  }
  const tally = el('div', 'tally');
  const n = report.findings.length;
  tally.append(el('span', null, `${n} finding${n === 1 ? '' : 's'}, ${report.score.toFixed(1)} tells per kiloword over ${report.words} words`));
  tally.append(el('span', `band ${report.band}`, report.band));
  out.append(tally);
}

function renderWho(a) {
  out.replaceChildren();
  if (!a.call) {
    const closest = a.ranked.slice(0, 2).map((c) => c.family).join(' or ');
    out.append(el('p', 'empty', `No catalog match. Closest would be ${closest}, and that is not close enough to say so.`));
  } else {
    const head = el('div', 'finding');
    head.append(el('span', 'f-id', a.call));
    head.append(el('span', null, `${Math.round(a.ranked[0].probability * 100)}% against ${Math.round(a.ranked[1].probability * 100)}% ${a.ranked[1].family}`));
    head.append(el('span', 'f-hit', ''));
    out.append(head);
    for (const r of a.receipts) {
      const row = el('div', 'finding');
      row.append(el('span', 'f-id', r.feature.slice(0, 2)));
      row.append(el('span', null, r.feature.slice(2)));
      row.append(el('span', 'f-hit', `${r.rate.toFixed(3)}/kw against ${r.baseline.toFixed(3)} baseline`));
      out.append(row);
    }
  }
  const tally = el('div', 'tally');
  tally.append(el('span', null, `among the ${a.catalog.length} classes in this catalog, ${a.matched} features matched`));
  out.append(tally);
}

async function run(action) {
  const text = input.value.trim();
  if (!text) {
    input.focus();
    return;
  }
  buttons.forEach((b) => (b.disabled = true));
  try {
    const t = await boot();
    if (action === 'who') await bootCatalog();
    const started = performance.now();
    if (action === 'fix') input.value = t.fix(input.value);
    if (action === 'who') {
      renderWho(JSON.parse(t.who(input.value, 0.54)));
    } else {
      renderLint(JSON.parse(t.lint_json(input.value)));
    }
    status.textContent = `${(performance.now() - started).toFixed(1)} ms, in this tab`;
  } catch (err) {
    out.replaceChildren(el('p', 'empty', 'The engine failed to load. Nothing was sent anywhere.'));
    status.textContent = 'failed';
    console.error(err);
  } finally {
    buttons.forEach((b) => (b.disabled = false));
  }
}

$('lint').addEventListener('click', () => run('lint'));
$('fix').addEventListener('click', () => run('fix'));
$('who').addEventListener('click', () => run('who'));
$('sample').addEventListener('click', () => {
  input.value = SAMPLE;
  run('lint');
});
