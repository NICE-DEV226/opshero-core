import sys,os,json,collections,math
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from signature import sign
def wilson(k,n,z=1.96):
    p=k/n; d=1+z*z/n; c=p+z*z/(2*n); a=z*math.sqrt(p*(1-p)/n+z*z/(4*n*n)); return round(100*(c-a)/d,1),round(100*(c+a)/d,1)
c=json.load(open("corpus.json"))
for r in c: r["sig"]=sign(r)
for stratum in ("NONTEST","TEST","ALL"):
    sub=[r for r in c if (stratum=="ALL" or r["stratum"]==stratum) and r["sig"]["T1"]]
    out={}
    for tier in ("T1","T2"):
        byrepo=collections.defaultdict(collections.Counter)
        for r in sub: byrepo[r["repo"]][r["sig"][tier]]+=1
        within=sum(1 for r in sub if byrepo[r["repo"]][r["sig"][tier]]>=2)
        allc=collections.Counter(r["sig"][tier] for r in sub)
        cross=sum(1 for r in sub if any(m["repo"]!=r["repo"] for m in sub if m["sig"][tier]==r["sig"][tier]))
        n=len(sub); out[tier]=(f"within-repo {within}/{n}={100*within/n:.1f}% {wilson(within,n)}", f"cross-repo {cross}/{n}={100*cross/n:.1f}%")
    print(stratum,out)
