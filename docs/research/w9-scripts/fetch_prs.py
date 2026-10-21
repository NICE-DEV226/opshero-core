#!/usr/bin/env python3
"""Stage 2: for each sampled repo, fetch (a) discovered agent PRs by number, (b) up to 100 PRs created in window via GraphQL search
(human comparison pool + any other agent PRs). Output raw/<owner>__<repo>.json. Read-only."""
import json, subprocess, sys, os, glob, time, random
W0, W1 = "2026-07-06", "2026-09-13"
os.makedirs("raw", exist_ok=True)
PRF = """number title createdAt closedAt mergedAt state merged isDraft headRefName headRefOid additions deletions changedFiles
 author{login __typename} bodyText
 files(first:50){nodes{path}}
 reviews{totalCount} comments{totalCount} reviewThreads{totalCount}
 commits(first:20){totalCount nodes{commit{oid committedDate message}}}
 baseRefName"""
def gql(q):
    for t in range(4):
        r = subprocess.run(["gh","api","graphql","-f","query="+q],capture_output=True,text=True)
        if r.returncode==0:
            try: return json.loads(r.stdout)
            except Exception: pass
        if "rate limit" in (r.stderr+r.stdout).lower() or "abuse" in (r.stderr+r.stdout).lower(): time.sleep(120)
        else: time.sleep(5)
    return None
def main():
    disc = {}
    for f in glob.glob("discover/*.json"):
        t = os.path.basename(f).split("_")[0]
        for p in json.load(open(f)):
            disc.setdefault(p["repository"]["nameWithOwner"],{})[p["number"]] = t
    repos = json.load(open("repos_selected.json"))
    for i, nwo in enumerate(repos):
        out = "raw/"+nwo.replace("/","__")+".json"
        if os.path.exists(out): continue
        o, n = nwo.split("/")
        nums = list(disc.get(nwo,{}))[:10]
        al = "".join(f"p{k}: pullRequest(number:{k}){{{PRF}}} " for k in nums)
        q = f'''query{{ repository(owner:"{o}",name:"{n}"){{ stargazerCount isFork createdAt isArchived owner{{__typename}} primaryLanguage{{name}}
          {al} }}
          search(query:"repo:{nwo} is:pr created:{W0}..{W1} sort:created-desc", type:ISSUE, first:40){{ issueCount nodes{{... on PullRequest{{{PRF}}}}} }} }}'''
        d = gql(q)
        if d is None or "data" not in d or d["data"] is None or d["data"].get("repository") is None:
            print("FAIL", nwo, str(d)[:150]); continue
        json.dump({"nwo":nwo,"disc":{str(k):v for k,v in disc.get(nwo,{}).items()},"data":d["data"]}, open(out,"w"))
        print(i, nwo, d["data"]["search"]["issueCount"], flush=True); time.sleep(1.5)
main()
