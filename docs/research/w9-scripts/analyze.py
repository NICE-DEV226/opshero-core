#!/usr/bin/env python3
"""Stage 4: metrics + tables. Run: python3 -I analyze.py  (reads prs.json repos.json sample.json ci/*.json; writes tables/*.csv, results.md)"""
import json, glob, re, math, random, statistics as st, datetime as dt, csv, os, collections
random.seed(1); os.makedirs("tables",exist_ok=True)
prs=json.load(open("prs.json")); repos=json.load(open("repos.json")); sample=json.load(open("sample.json"))
ci={}
for f in glob.glob("ci/*.json"):
    nwo=os.path.basename(f)[:-5].replace("__","/",1); ci[nwo]=json.load(open(f))
T=lambda s: dt.datetime.fromisoformat(s.replace("Z","+00:00"))
CI_P=re.compile(r"^\.github/|\.circleci/|\.gitlab-ci|Jenkinsfile|azure-pipelines|\.buildkite/")
TEST_P=re.compile(r"(^|/)(tests?|__tests__|spec|e2e)/|\.(test|spec)\.[a-z]+$|(^|/)test_[^/]*$|_test\.[a-z]+$")
CFG_P=re.compile(r"(^|/)(\.[a-z]+rc(\.[a-z]+)?|[^/]*config[^/]*|tsconfig[^/]*|pyproject\.toml|package\.json|Makefile|Dockerfile[^/]*|docker-compose[^/]*|\.env[^/]*|[^/]+\.(ya?ml|toml|ini|cfg))$")
def wilson(k,n,z=1.96):
    if n==0: return (float("nan"),)*2
    p=k/n; d=1+z*z/n; c=(p+z*z/(2*n))/d; h=z*math.sqrt(p*(1-p)/n+z*z/(4*n*n))/d; return (c-h,c+h)
def boot_med(x,B=2000):
    if len(x)<2: return (float("nan"),)*2
    m=sorted(st.median(random.choices(x,k=len(x))) for _ in range(B)); return (m[int(.025*B)],m[int(.975*B)])
def ev(c):  # per-commit CI verdict
    s=[x for x in c["checkSuites"]["nodes"] if x["conclusion"] in("SUCCESS","FAILURE","TIMED_OUT","STARTUP_FAILURE","ACTION_REQUIRED")]
    if not s: return None,None
    cs={x["conclusion"] for x in s}
    if cs&{"FAILURE","TIMED_OUT","STARTUP_FAILURE"}: v="fail"
    elif "ACTION_REQUIRED" in cs: v="approval"
    else: v="pass"
    return v, max(x["updatedAt"] for x in s)
rows=[]
for k,p in prs.items():
    if p["cat"]=="other_bot": continue
    nwo=p["nwo"]; r=repos[nwo]; c=ci.get(nwo,{}).get(f"p{p['number']}")
    row=dict(key=k,nwo=nwo,cat=p["cat"],big=r["stargazerCount"]>100,stars=r["stargazerCount"],state="merged" if p["merged"] else p["state"].lower(),
        size=p["additions"]+p["deletions"],nfiles=p["changedFiles"],threads=p["reviewThreads"]["totalCount"],reviews=p["reviews"]["totalCount"],
        comments=p["comments"]["totalCount"],created=p["createdAt"])
    paths=[f["path"] for f in p["files"]["nodes"]]
    row["touch_ci"]=any(CI_P.search(x) for x in paths); row["touch_test"]=any(TEST_P.search(x) for x in paths); row["touch_cfg"]=any(CFG_P.search(x) for x in paths)
    row["touch_any"]=row["touch_ci"] or row["touch_test"] or row["touch_cfg"]; row["files_truncated"]=p["changedFiles"]>50
    row["ttm_h"]=(T(p["mergedAt"])-T(p["createdAt"])).total_seconds()/3600 if p["merged"] else None
    row["has_ci_data"]=c is not None; row["first_ci"]=None; row["push_before_green"]=None; row["ttg_h"]=None; row["ever_green"]=None; row["fu_fix"]=None; row["revert"]=None
    if c:
        vs=[ev(n["commit"]) for n in c["commits"]["nodes"]]
        seq=[(v,t) for v,t in vs if v]
        row["first_ci"]=seq[0][0] if seq else "none"
        if seq:
            g=next((i for i,(v,_) in enumerate(seq) if v=="pass"),None); row["ever_green"]=g is not None
            if g is not None:
                row["push_before_green"]=g; row["ttg_h"]=(T(seq[g][1])-T(p["createdAt"])).total_seconds()/3600
        if p["merged"]:
            h=ci[nwo].get(f"h{p['number']}")
            if h and h.get("target"):
                mt=T(p["mergedAt"]); fu=rv=False; ref="#%d"%p["number"]
                for m in h["target"]["history"]["nodes"]:
                    msg=m["message"]; first=msg.split("\n")[0]
                    if (T(m["committedDate"])-mt).total_seconds()<300 and first.rstrip().endswith("(%s)"%ref): continue
                    if (T(m["committedDate"])-mt).total_seconds()<=0: continue
                    refd=re.search(re.escape(ref)+r"(?!\d)",msg)
                    if refd and re.match(r"\s*revert",first,re.I): rv=True
                    elif refd and re.search(r"\b(fix|hotfix|follow[- ]?up|regression)\b",msg,re.I): fu=True
                    elif re.match(r"\s*revert",first,re.I) and p["title"][:40].lower() in msg.lower(): rv=True
                row["fu_fix"]=fu; row["revert"]=rv
    rows.append(row)
GRP={"copilot":["copilot"],"devin":["devin"],"cursor_app":["cursor"],"claude_app":["claude_bot"],"claude_code_marker(human acct)":["claude_code_human"],
     "other_marked(codex/cursor branch)":["codex_branch_human","cursor_marker_human"],"AGENT_APPS(all 4 bots)":["copilot","devin","cursor","claude_bot"],"human":["human"]}
def sub(g,big=None): return [r for r in rows if r["cat"] in GRP[g] and (big is None or r["big"]==big)]
def prop(rs,f,cond=lambda r:True):
    rs=[r for r in rs if cond(r) and r[f] is not None]; n=len(rs); k=sum(1 for r in rs if r[f] is True or r[f]==f_val.get(f,True)) if False else sum(1 for r in rs if r[f]); lo,hi=wilson(k,n); return n,k,lo,hi
def fmt(n,k,lo,hi): return f"{k}/{n} = {100*k/n:.0f}% [{100*lo:.0f}-{100*hi:.0f}]" if n else "n=0"
def med(x): 
    x=[v for v in x if v is not None]; 
    return f"{st.median(x):.1f} [{boot_med(x)[0]:.1f}-{boot_med(x)[1]:.1f}] (n={len(x)})" if len(x)>1 else f"n={len(x)}"
out=[]
def table(title,big):
    out.append(f"\n### {title}\n")
    out.append("| group | n PR | repos | merged | closed-unmerged | open | first-run CI fail (of PRs w/ CI verdict) | first-run needs approval | ever green <=20 commits | median pushes before 1st green | median TTG h | median TTM h (merged) | touches CI/test/cfg | touches test | review threads>=1 | fu-fix/revert 7d (merged, checked) |")
    out.append("|"+"---|"*16)
    for g in GRP:
        rs=sub(g,big); n=len(rs)
        if n==0: continue
        res=[]; 
        for s in ("merged","closed","open"): res.append(fmt(n,sum(r["state"]==s for r in rs),*wilson(sum(r["state"]==s for r in rs),n)))
        w=[r for r in rs if r["first_ci"] in("pass","fail","approval")]
        ff=fmt(len(w),sum(r["first_ci"]=="fail" for r in w),*wilson(sum(r["first_ci"]=="fail" for r in w),len(w)))
        ap=fmt(len(w),sum(r["first_ci"]=="approval" for r in w),*wilson(sum(r["first_ci"]=="approval" for r in w),len(w)))
        eg=[r for r in rs if r["ever_green"] is not None]; egs=fmt(len(eg),sum(r["ever_green"] for r in eg),*wilson(sum(r["ever_green"] for r in eg),len(eg)))
        pb=med([r["push_before_green"] for r in rs]); tg=med([r["ttg_h"] for r in rs]); tm=med([r["ttm_h"] for r in rs])
        ta=fmt(*prop(rs,"touch_any")); tt=fmt(*prop(rs,"touch_test")); rt=fmt(n,sum(r["threads"]>=1 for r in rs),*wilson(sum(r["threads"]>=1 for r in rs),n))
        fu=[r for r in rs if r["fu_fix"] is not None]; k=sum(r["fu_fix"] or r["revert"] for r in fu); fus=fmt(len(fu),k,*wilson(k,len(fu)))
        out.append(f"| {g} | {n} | {len({r['nwo'] for r in rs})} | {res[0]} | {res[1]} | {res[2]} | {ff} | {ap} | {egs} | {pb} | {tg} | {tm} | {ta} | {tt} | {rt} | {fus} |")
table("All stars",None); table("Repos >100 stars",True); table("Repos <=100 stars",False)
# repo-matched differences (equal-weight per repo, cluster bootstrap)
def matched(g,f,cond=lambda r:True,big=None):
    byr=collections.defaultdict(lambda:[[],[]])
    for r in rows:
        if big is not None and r["big"]!=big: continue
        if not cond(r) or r[f] is None: continue
        v=float(r[f] if not isinstance(r[f],str) else r[f]=="x")
        if r["cat"] in GRP[g]: byr[r["nwo"]][0].append(v)
        elif r["cat"]=="human": byr[r["nwo"]][1].append(v)
    ds=[st.mean(a)-st.mean(h) for a,h in byr.values() if a and h]
    if len(ds)<5: return f"n_repos={len(ds)}"
    m=sorted(st.mean(random.choices(ds,k=len(ds))) for _ in range(2000))
    return f"{100*st.mean(ds):+.1f} pp [{100*m[50]:+.1f}, {100*m[1949]:+.1f}] (repos={len(ds)})"
out.append("\n### Repo-matched differences (agent minus human, pp; equal weight per repo; cluster-bootstrap 95% CI over repos)\n")
out.append("| group | merged | first-run CI fail* | touches test | touches CI/test/cfg | review thread>=1 | size>200 lines |"); out.append("|---|---|---|---|---|---|---|")
for r in rows: r["merged_f"]=r["state"]=="merged"; r["ci_fail"]=(r["first_ci"]=="fail") if r["first_ci"] in("pass","fail") else None; r["thr1"]=r["threads"]>=1; r["big_pr"]=r["size"]>200
for g in ["copilot","devin","cursor_app","claude_app","claude_code_marker(human acct)","AGENT_APPS(all 4 bots)"]:
    out.append(f"| {g} | "+" | ".join(matched(g,f) for f in ["merged_f","ci_fail","touch_test","touch_any","thr1","big_pr"])+" |")
out.append("\n*first-run CI fail restricted to pass/fail verdicts (approval-gated runs excluded).\n")
# composition
out.append("\n### Star-stratum composition of agent PRs in sample\n"); out.append("| group | n | >100 stars | <=10 stars | median stars |"); out.append("|---|---|---|---|---|")
for g in GRP:
    rs=sub(g); n=len(rs)
    if n: out.append(f"| {g} | {n} | {fmt(n,sum(r['big'] for r in rs),*wilson(sum(r['big'] for r in rs),n))} | {fmt(n,sum(r['stars']<=10 for r in rs),*wilson(sum(r['stars']<=10 for r in rs),n))} | {st.median(r['stars'] for r in rs)} |")
out.append("\nMedian PR size (additions+deletions) and created-date median:\n"); out.append("| group | median size | median created |"); out.append("|---|---|---|")
for g in GRP:
    rs=sub(g)
    if rs: out.append(f"| {g} | {med([r['size'] for r in rs])} | {sorted(r['created'] for r in rs)[len(rs)//2][:10]} |")
out.append(f"\nCI data coverage: {sum(r['has_ci_data'] for r in rows)}/{len(rows)} PRs; PRs with >50 files (file list truncated): {sum(r['files_truncated'] for r in rows)}; repos in sample {len({r['nwo'] for r in rows})}.")
open("results.md","w").write("\n".join(out)); 
with open("tables/pr_rows.csv","w") as f:
    w=csv.DictWriter(f,fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
print("\n".join(out))
