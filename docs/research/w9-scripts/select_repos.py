#!/usr/bin/env python3
"""Stage 1b: pick repos. Discovery counts per repo; stars fetched in batches via GraphQL aliases.
Seeded random sample: up to N_BIG repos with >100 stars + N_SMALL repos <=100 stars, requiring >=1 discovered agent PR."""
import json, glob, os, random, subprocess, collections
random.seed(42)
N_BIG, N_SMALL = 160, 160
cnt = collections.defaultdict(lambda: collections.Counter())
for f in glob.glob("discover/*.json"):
    t = os.path.basename(f).split("_")[0]
    for p in json.load(open(f)): cnt[p["repository"]["nameWithOwner"]][t]+=1
repos = sorted(cnt)
stars = {}
for i in range(0,len(repos),50):
    b = repos[i:i+50]
    q = "query{"+"".join(f'r{k}: repository(owner:"{r.split("/")[0]}",name:"{r.split("/")[1]}"){{stargazerCount}} ' for k,r in enumerate(b))+"}"
    r = subprocess.run(["gh","api","graphql","-f","query="+q],capture_output=True,text=True)
    d = json.loads(r.stdout).get("data") or {}
    for k,rr in enumerate(b): stars[rr] = (d.get(f"r{k}") or {}).get("stargazerCount")
json.dump({"counts":{r:dict(cnt[r]) for r in repos},"stars":stars}, open("discovered_repos.json","w"))
big=[r for r in repos if (stars[r] or 0)>100]; small=[r for r in repos if stars[r] is not None and stars[r]<=100]
print("discovered repos",len(repos),"big",len(big),"small",len(small))
sel = random.sample(big,min(N_BIG,len(big)))+random.sample(small,min(N_SMALL,len(small)))
json.dump(sel, open("repos_selected.json","w")); print("selected",len(sel))
