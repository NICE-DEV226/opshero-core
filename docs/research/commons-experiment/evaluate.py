import sys,os,json,re,math,random,collections
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from signature import sign
def wilson(k,n,z=1.96):
    if n==0: return (0.0,0.0)
    p=k/n; d=1+z*z/n; c=p+z*z/(2*n); a=z*math.sqrt(p*(1-p)/n+z*z/(4*n*n)); return ((c-a)/d,(c+a)/d)
c=json.load(open("corpus.json"))
for r in c: r["sig"]=sign(r)
def b1(r): return re.sub(r"[0-9a-f]{7,}|\d+","#",(r.get("step") or "").lower())
def b2(r): return r["sig"]["primary"]
dev=[r for r in c if r["split"]=="DEV"]; test=[r for r in c if r["split"]=="TEST"]
idx={k:collections.defaultdict(list) for k in ("T1","T2","B1","B2")}
keyf={"T1":lambda r:r["sig"]["T1"],"T2":lambda r:r["sig"]["T2"],"B1":b1,"B2":b2}
for r in dev:
    for k in idx:
        key=keyf[k](r)
        if key: idx[k][key].append(r)
res={}; pairs=[]
for stratum in ("NONTEST","TEST","ALL"):
    sub=[r for r in test if stratum=="ALL" or r["stratum"]==stratum]
    row={"n":len(sub),"unsignable":sum(1 for r in sub if r["sig"]["T1"] is None)}
    for k in idx:
        hit=0
        for r in sub:
            key=keyf[k](r)
            if key and any(m["repo"]!=r["repo"] for m in idx[k].get(key,[])): hit+=1
        lo,hi=wilson(hit,len(sub)); row[k]=(hit,len(sub),round(100*hit/max(1,len(sub)),1),round(100*lo,1),round(100*hi,1))
    res[stratum]=row
# pairs for judging: every T1 match + up to 60 T2-only matches (non-test stratum prioritized), 1 nearest DEV partner each
def partner(r,k):
    key=keyf[k](r); ms=[m for m in idx[k].get(key,[]) if m["repo"]!=r["repo"]] if key else []
    return ms[0] if ms else None
seen=set()
for k in ("T1","T2"):
    for r in test:
        p=partner(r,k)
        if p and (r["run"],p["run"]) not in seen:
            seen.add((r["run"],p["run"]))
            pairs.append({"tier":k,"stratum":r["stratum"],"a":{"repo":r["repo"],"step":r["step"],"excerpt":r["excerpt"][-12:],"err":r["errors"][:1]},
                          "b":{"repo":p["repo"],"step":p["step"],"excerpt":p["excerpt"][-12:],"err":p["errors"][:1]}})
t2only=[p for p in pairs if p["tier"]=="T2"]; random.Random(3).shuffle(t2only)
keep=[p for p in pairs if p["tier"]=="T1"]+t2only[:60]
for i,p in enumerate(keep): p["id"]=i
json.dump(keep,open("pairs_to_judge.json","w"))
# Good-Turing coverage on ALL logs by stratum and tier (signable only)
gt={}
for stratum in ("NONTEST","TEST","ALL"):
    sub=[r for r in c if (stratum=="ALL" or r["stratum"]==stratum) and r["sig"]["T1"]]
    for k in ("T1","T2"):
        cnt=collections.Counter(keyf[k](r) for r in sub); n1=sum(1 for v in cnt.values() if v==1); N=len(sub)
        top5=sum(v for _,v in cnt.most_common(5))
        gt[f"{stratum}/{k}"]=dict(N=N,types=len(cnt),singletons=n1,GT_coverage=round(1-n1/N,3) if N else None,top5_share=round(top5/N,3) if N else None)
json.dump({"results":res,"good_turing":gt,"pairs_n":len(keep),"t1_pairs":sum(1 for p in keep if p["tier"]=="T1")},open("results_test.json","w"),indent=1)
print(json.dumps(res,indent=1)); print(json.dumps(gt,indent=1)); print("pairs to judge:",len(keep))
