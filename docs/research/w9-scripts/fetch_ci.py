#!/usr/bin/env python3
"""Stage 3: per sampled PR, first-20-commit check suites (CI) and, for merged PRs, default-branch commits in the 7 days after merge.
Reads sample.json (list of [nwo, number, mergedAt|None]); writes ci/<owner>__<repo>.json."""
import json, subprocess, time, os, datetime as dt, collections
os.makedirs("ci", exist_ok=True)
def gql(q):
    for t in range(4):
        r = subprocess.run(["gh","api","graphql","-f","query="+q],capture_output=True,text=True)
        if r.returncode==0:
            try:
                d=json.loads(r.stdout)
                if d.get("data"): return d["data"]
            except Exception: pass
        if "rate limit" in (r.stderr+r.stdout).lower(): time.sleep(120)
        else: time.sleep(5)
    return None
sample = json.load(open("sample.json")); by = collections.defaultdict(list)
for nwo,num,m in sample: by[nwo].append((num,m))
for i,(nwo,items) in enumerate(sorted(by.items())):
    out="ci/"+nwo.replace("/","__")+".json"
    if os.path.exists(out): continue
    o,n = nwo.split("/"); res={}
    for b in range(0,len(items),6):
        chunk=items[b:b+6]; al=""
        for num,m in chunk:
            al += 'p%d: pullRequest(number:%d){ commits(first:20){nodes{commit{oid committedDate checkSuites(first:12){nodes{conclusion status createdAt updatedAt app{slug}}}}}}} ' % (num,num)
            if m:
                s_=m; e=(dt.datetime.fromisoformat(m.replace("Z","+00:00"))+dt.timedelta(days=7)).strftime("%Y-%m-%dT%H:%M:%SZ")
                al += 'h%d: defaultBranchRef{target{... on Commit{history(since:"%s",until:"%s",first:60){nodes{message committedDate}}}}} ' % (num,s_,e)
        d=gql(f'query{{repository(owner:"{o}",name:"{n}"){{{al}}}}}')
        if d and d.get("repository"): res.update(d["repository"])
        time.sleep(1)
    json.dump(res,open(out,"w")); print(i,nwo,len(res),flush=True)
