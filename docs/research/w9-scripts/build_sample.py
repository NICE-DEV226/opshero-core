#!/usr/bin/env python3
"""Stage 2b: parse raw/, build prs.json (dedup by nwo#number) + sample.json for CI stage (all agent-ish PRs, <=25 random human per repo)."""
import json, glob, random, sys
sys.path.insert(0,".")
from classify import category
random.seed(7)
prs={}; repo={}
for f in glob.glob("raw/*.json"):
    j=json.load(open(f)); d=j["data"]; nwo=j["nwo"]; r=d["repository"]
    repo[nwo]={k:r[k] for k in ("stargazerCount","isFork","createdAt","isArchived")}|{"owner":r["owner"]["__typename"],"search_total":d["search"]["issueCount"]}
    nodes=[v for k,v in r.items() if k.startswith("p") and isinstance(v,dict) and v.get("number")]+[n for n in d["search"]["nodes"] if n]
    for p in nodes:
        if not ("2026-07-06"<=p["createdAt"][:10]<="2026-09-13"): continue
        p["nwo"]=nwo; p["cat"]=category(p); prs[f"{nwo}#{p['number']}"]=p
json.dump(prs,open("prs.json","w")); json.dump(repo,open("repos.json","w"))
by={}
for k,p in prs.items(): by.setdefault(p["nwo"],[]).append(p)
sample=[]
for nwo,ps in by.items():
    hum=[p for p in ps if p["cat"]=="human"]; random.shuffle(hum)
    ag=[p for p in ps if p["cat"] not in("human","other_bot")]; random.shuffle(ag); keep=ag[:12]+hum[:12]
    sample+= [[nwo,p["number"],p["mergedAt"]] for p in keep]
json.dump(sample,open("sample.json","w"))
import collections; print(len(prs),collections.Counter(p["cat"] for p in prs.values()),len(sample))
