"""Makes the FVH_PROTO file for `shots.rs` `against_the_prototype`: 150 sets of Similar shots (their
names, the Best one, whether a close call) and the numbers of 300 photos (brightness, edges,
fingerprint, Junk) from the Photos prototype's page (its index.html), kept by the maintainer.

  python3 tests/proto_sets.py PROTOTYPE_FOLDER .scratch/proto.json
"""
import json
import random
import re
import sys
from pathlib import Path

OUT = Path(sys.argv[1])
html = (OUT / "index.html").read_text(encoding="utf-8")
# P row: [i, name, video, tw, th, ts, group, groupKind, best, junk, dur, copyOf, sharpness, pixels, why, close]
P = json.loads(re.search(r"const D = (\{.*?\});\n", html).group(1))["P"]
scan = json.loads((OUT / "scan.json").read_text())
groups = {}
for r in P:
    if r[6]:
        groups.setdefault(r[6], []).append(r)
random.seed(2)
sets = [{"names": [r[1] for r in g], "best": next(r[1] for r in g if r[8]), "close": any(r[15] for r in g)}
        for g in random.sample(list(groups.values()), 150)]
junk = {r[1]: r[9] for r in P}
pics = [n for n, v in scan.items() if not n.lower().endswith((".mp4", ".3gp")) and "err" not in v and "tw" in v]
stats = {n: {"lum": scan[n]["lum"], "sharp": scan[n]["sharp"], "dh": scan[n]["dh"], "junk": junk.get(n, "")} for n in random.sample(pics, 300)}
Path(sys.argv[2]).write_text(json.dumps({"sets": sets, "stats": stats}, ensure_ascii=False), encoding="utf-8")
