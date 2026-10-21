import sys,os,json,re,random
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from signature import sign, normalize
c=json.load(open("corpus.json"))
STOP=set("error errors failed failure process completed exit code the and for with not url path hex ver step run".split())
def toks(r):
    s=sign(r); p=s["primary"] or (r["errors"][0] if r["errors"] else "")
    t=set(re.findall(r"[a-z_][a-z0-9_]{2,}",normalize(p)+" "+(r.get("step") or "").lower()))
    return t-STOP
dev=[r for r in c if r["split"]=="DEV"]; test=[r for r in c if r["split"]=="TEST" and r["stratum"]=="NONTEST"]
D=[(r,toks(r)) for r in dev]
done=json.load(open("judge_key_PRIVATE.json"))
cand=[]
for r in test:
    t=toks(r)
    if len(t)<3: continue
    best=max(((len(t&d)/len(t|d),m) for m,d in D if m["repo"]!=r["repo"] and len(d)>=3),default=(0,None),key=lambda x:x[0])
    if best[1] is not None: cand.append((best[0],r,best[1]))
cand.sort(key=lambda x:-x[0])
top=cand[:50]
print("test nontest logs",len(test),"with candidates",len(cand),"| similarity of top50: min",round(top[-1][0],2),"max",round(top[0][0],2),"| frac>=0.5:",sum(1 for s,_,_ in cand if s>=0.5)/len(test))
def fmt(x): return {"repo":x["repo"],"failing_step":x["step"],"last_log_lines":[l[:170] for l in x["excerpt"][-8:]],"error_annotation":[e[:170] for e in x["errors"][:1]]}
blind=[]; key={}
random.Random(5).shuffle(top)
for i,(s,a,b) in enumerate(top):
    blind.append({"id":i,"A":fmt(a),"B":fmt(b)}); key[i]=round(s,2)
json.dump(blind,open("judge_fuzzy.json","w")); json.dump(key,open("judge_fuzzy_key_PRIVATE.json","w"))
