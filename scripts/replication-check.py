#!/usr/bin/env python3
"""Is our harvest comparable to the reference corpus we compare against?

Every cross-corpus claim in this repo assumes our samples and locuslab's are
drawn the same way. That assumption is free to make and expensive to be wrong
about, so it gets a control: gh/gpt-4o-2024-08-06 is the SAME model locuslab
used, asked the same prompts by our harvester. If the two agree, differences
elsewhere are the models changing. If they disagree, they are us.

Result on 2026-07-28, 14 matched prompts:
    ours     414 words median
    locuslab 428 words median
    ratio    1.04

Which settles a question the numbers raised. 2026 models answer far shorter
than 2024 ones on identical prompts (gemini-3-flash 72 words against
gemini-1.5-pro's 446), and this says that is the models, not the harvest.
Terseness is a real generational shift and part of the fingerprint.

Usage: python3 scripts/replication-check.py [n_prompts]
"""
import json
import statistics
import subprocess
import sys
import pathlib

MODEL = 'gh/gpt-4o-2024-08-06'  # the exact id in locuslab/llm-idiosyncrasies
URL = 'http://127.0.0.1:20128/v1/chat/completions'

root = pathlib.Path(__file__).parent.parent
n = int(sys.argv[1]) if len(sys.argv) > 1 else 14
prompts = [json.loads(l) for l in (root / 'corpora/prompts.jsonl').open()][:n]

ref = {}
for line in (root / 'corpora/locuslab.jsonl').open():
    r = json.loads(line)
    if r['family'] == 'gpt-4':
        ref[r['prompt_id']] = len(r['text'].split())

ours, theirs = [], []
for p in prompts:
    body = json.dumps({'model': MODEL, 'stream': False, 'max_tokens': 1024,
                       'messages': [{'role': 'user', 'content': p['prompt']}]})
    out = subprocess.run(
        ['curl', '-s', '-m', '120', URL, '-H', 'Content-Type: application/json', '-d', body],
        capture_output=True, text=True).stdout
    try:
        text = json.loads(out)['choices'][0]['message']['content']
    except Exception:
        print(f'  {p["prompt_id"]}: no answer, skipped')
        continue
    if text.strip() and p['prompt_id'] in ref:
        ours.append(len(text.split()))
        theirs.append(ref[p['prompt_id']])

if not ours:
    raise SystemExit('no matched samples, is the gateway up?')

a, b = statistics.median(ours), statistics.median(theirs)
print(f'{MODEL}, {len(ours)} matched prompts')
print(f'  ours     {a:5.0f} words median')
print(f'  locuslab {b:5.0f} words median')
print(f'  ratio    {b / a:.2f}')
print('\nA ratio near 1 means the harvests are comparable and any difference'
      '\nelsewhere belongs to the models. Far from 1 means it belongs to us.')
