#!/usr/bin/env python3
"""Have the models converged? Built to be able to answer no.

Class count, sample size and response length all move attribution accuracy on
their own, so a comparison that does not fix them measures nothing. This cuts
both eras to the same shape and leaves only the generation varying, then does
it three ways because the aggregate hides the interesting part.

  catalog   four families, 295 samples each, plus an equal rejection class
  vendors   only Anthropic, Google and OpenAI, present in both eras
  pairs     two classes at a time, which is where the story actually is

Then eval each output identically, and vary --truncate to separate "says
different things" from "says a different amount":

  tellem eval corpora/catalog-2026.jsonl --top-k 2000 --epochs 16 --truncate 50
"""
import collections
import itertools
import json
import pathlib
import random

root = pathlib.Path(__file__).parent.parent
corp = root / 'corpora'
old = [json.loads(l) for l in (corp / 'locuslab.jsonl').open()]
new = [json.loads(l) for l in (corp / 'harvest.jsonl').open()]
human = [json.loads(l) for l in (corp / 'mage-human.jsonl').open()]
random.Random(7).shuffle(human)

ERAS = {
    '2024': (old, {'claude': 'claude-3', 'gemini': 'gemini-1', 'gpt': 'gpt-4', 'other': 'deepseek'}),
    '2026': (new, {'claude': 'claude-4', 'gemini': 'gemini-3', 'gpt': 'gpt-5', 'other': 'gpt-oss'}),
}
N = 280


def write(path, groups, rejection=True):
    with path.open('w') as fh:
        for rows in groups:
            for r in rows:
                fh.write(json.dumps({'family': r['family'], 'prompt_id': r['prompt_id'],
                                     'text': r['text']}) + '\n')
        if rejection:
            for r in human[:N]:
                fh.write(json.dumps({'family': 'unmatched', 'prompt_id': '',
                                     'text': r['text']}) + '\n')
    print(f'  {path.name}')


def take(rows, family):
    sel = [r for r in rows if r['family'] == family]
    return random.Random(11).sample(sel, min(N, len(sel)))


for era, (rows, fam) in ERAS.items():
    print(era)
    write(corp / f'catalog-{era}-matched.jsonl', [take(rows, f) for f in fam.values()])
    write(corp / f'vendors-{era}.jsonl',
          [take(rows, fam[k]) for k in ('claude', 'gemini', 'gpt')])
    for a, b in itertools.combinations(('claude', 'gemini', 'gpt'), 2):
        write(corp / f'pair-{era}-{a}{b}.jsonl',
              [take(rows, fam[a]), take(rows, fam[b])], rejection=False)

counts = collections.Counter(r['family'] for r in new)
print('\nharvest holds:', dict(counts))
