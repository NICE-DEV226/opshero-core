import sys,os,json,collections
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from signature import sign
c=json.load(open("corpus.json"))
dev=[r for r in c if r["split"]=="DEV"]
S=[(r,sign(r)) for r in dev]
print("DEV logs",len(dev),"unsignable(generic only):",sum(1 for r,s in S if s["T1"] is None))
t1=collections.defaultdict(set); t2=collections.defaultdict(set)
for r,s in S:
    if s["T1"]: t1[s["T1"]].add(r["repo"]); t2[s["T2"]].add(r["repo"])
print("T1 types",len(t1),"cross-repo T1 (>=2 repos):",sum(1 for v in t1.values() if len(v)>=2))
print("T2 types",len(t2),"cross-repo T2 (>=2 repos):",sum(1 for v in t2.values() if len(v)>=2))
print("top T1 cross-repo:",[(k[0],k[1][:70],len(v)) for k,v in sorted(t1.items(),key=lambda kv:-len(kv[1])) if len(v)>=2][:8])
print("T2 class dist:",collections.Counter(k[1] for k in t2).most_common())
ex=[(s["T1"],r["repo"]) for r,s in S if s["T1"]][:12]
for k,rp in ex: print("  e.g.",rp,"->",k[0],"|",k[1][:90])
