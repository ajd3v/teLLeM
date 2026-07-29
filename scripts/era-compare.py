#!/usr/bin/env python3
"""Does a stylometric fingerprint survive two model generations?

The clean version of that question needs everything except the era held fixed,
so this builds two corpora that differ ONLY in when the models were released:

  2024  claude-3-5-sonnet  vs  gemini-1.5-pro     (locuslab/llm-idiosyncrasies)
  2026  claude-sonnet-4-6  vs  gemini-3-flash     (our own harvest)

Same two vendors, the same prompt ids in both, the same sample count, the same
two-class task. The harvest battery deliberately reuses the reference corpus's
prompts, which is what makes this comparable at all.

Run, then eval each output with the same flags:
  tellem eval corpora/era-2024.jsonl --top-k 2000 --epochs 20 --holdout 30
"""
import collections
import json
import pathlib

root = pathlib.Path(__file__).parent.parent
harvest = [json.loads(l) for l in (root / 'corpora/harvest.jsonl').open()]
old = [json.loads(l) for l in (root / 'corpora/locuslab.jsonl').open()]

new_by = collections.defaultdict(dict)
for r in harvest:
    new_by[r['family']][r['prompt_id']] = r['text']
old_by = collections.defaultdict(dict)
for r in old:
    old_by[r['family']][r['prompt_id']] = r['text']

ids = sorted(
    set(new_by['claude-4']) & set(new_by['gemini-3'])
    & set(old_by['claude-3']) & set(old_by['gemini-1'])
)
print(f'{len(ids)} prompts answered by all four models')

for era, src, pair in [('2024', old_by, ('claude-3', 'gemini-1')),
                       ('2026', new_by, ('claude-4', 'gemini-3'))]:
    out = root / f'corpora/era-{era}.jsonl'
    with out.open('w') as fh:
        for fam in pair:
            for pid in ids:
                fh.write(json.dumps(
                    {'family': fam, 'prompt_id': pid, 'text': src[fam][pid]}) + '\n')
    print(f'{era}: {len(ids) * 2} samples -> {out.name}')

print("""
Known confounds, since this is a two-point comparison and not a study:
  - gemini-3-flash is a faster tier than gemini-1.5-pro, so vendor is matched
    but model tier is not.
  - the 2026 half was harvested with max_tokens 1024, the 2024 half's
    generation parameters are not published.
  - roughly 130 held-out samples per era puts the 95% interval near 5 points,
    which is wider than the gap it is measuring.""")
