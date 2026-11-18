"""oracle/parse_les.py: parse a <prefix>.les trace (LOQ_TRACE_LES, loqref.c) into one JSON object per utterance.

Each record: {"b": "24 hex bytes", "s": text at +4 (latin-1 str) or None, "s0": tag at +0}
Utterance keys: stage name -> {"in": [records], "out": [records]} for Les* stages,
"tp": [[word, idx, [codes]]], "tc": [[code, p3, p4]], "fon": transcription.
"""
import json, re, sys

def unesc(s):
    out = bytearray()
    i = 0
    while i < len(s):
        c = s[i]
        if c == '\\':
            if s[i + 1] == 'x':
                out.append(int(s[i + 2:i + 4], 16)); i += 4; continue
            out.append(ord(s[i + 1])); i += 2; continue
        out.append(ord(c)); i += 1
    return out.decode('latin-1')

STR = re.compile(r' (\w+)="((?:[^"\\]|\\.)*)"')

def parse(path, fonlist=None):
    fons = []
    if fonlist:
        fons = [l.rstrip('\n').split('\t', 1) for l in open(fonlist, encoding='utf-8', errors='replace')]
    utts = []
    cur = None
    stage = None
    side = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('> LesAnalizza'):
            cur = {}
            utts.append(cur)
        if cur is None:
            continue
        if line.startswith('> ') or line.startswith('< '):
            name = line[2:].split(' ', 1)[0]
            d = dict((k, unesc(v)) for k, v in STR.findall(line))
            if name.startswith('Les') and name not in ('LesPronunciabile', 'LesNormalizzaAccento', 'LesGetSymbolName'):
                stage = name
                side = 'in' if line[0] == '>' else 'out'
                cur.setdefault(stage, {})[side] = []
                m = re.search(r' ctx=([0-9a-f,]+)', line)
                if m and side == 'in':
                    cur[stage]['ctx'] = [int(x, 16) for x in m.group(1).split(',')]
            else:
                stage = None
            if name == 'FonTrasParola' and line[0] == '>':
                cur.setdefault('tp', []).append([d['word'], int(re.search(r' idx=(\d+)', line).group(1)), None])
            elif name == 'FonTrasParola':
                m = re.search(r' out=([0-9a-f,]*)', line)
                cur['tp'][-1][2] = [int(x, 16) for x in m.group(1).split(',') if x]
            elif name == 'FonTrasConfini' and line[0] == '>':
                m = re.search(r' code=([0-9a-f]+) p3=(-?\d+) p4=(-?\d+)', line)
                cur.setdefault('tc', []).append([int(m.group(1), 16), int(m.group(2)), int(m.group(3))])
        elif line.startswith('  W ') and stage:
            parts = line.split(' ')
            hexb = ''.join(parts[4:10])
            d = dict((k, unesc(v)) for k, v in STR.findall(line))
            cur[stage][side].append({'b': hexb, 's': d.get('s4'), 's0': d.get('s0')})
    return utts

if __name__ == '__main__':
    utts = parse(sys.argv[1])
    with open(sys.argv[2], 'w', encoding='utf-8') as f:
        for u in utts:
            f.write(json.dumps(u, ensure_ascii=False) + '\n')
    print(len(utts), 'utterances')
