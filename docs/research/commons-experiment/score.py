import json, math, collections
def wilson(k,n,z=1.96):
    if n==0: return (0,0)
    p=k/n; d=1+z*z/n; c=p+z*z/(2*n); a=z*math.sqrt(p*(1-p)/n+z*z/(4*n*n)); return ((c-a)/d,(c+a)/d)
key=json.load(open("judge_key_PRIVATE.json")); res=json.load(open("results_test.json"))["results"]
def lv(f):
    d=json.load(open(f)); m={i:0 for i in range(70)}
    for L,ids in d.items():
        for i in ids: m[i]=int(L)
    return m
j1,j2=lv("judge1.json"),lv("judge2.json")
agree=sum(1 for i in range(70) if j1[i]==j2[i]); print("exact agreement",agree,"/70 =",round(agree/70,3))
for name,rule in (("conservative(both>=2)",lambda a,b:a>=2 and b>=2),("liberal(either>=2)",lambda a,b:a>=2 or b>=2)):
    print("==",name)
    for tier in ("T1","T2"):
        for st in ("NONTEST","TEST"):
            ids=[int(i) for i,v in key.items() if v["tier"]==tier and v["stratum"]==st]
            if not ids: continue
            valid=sum(1 for i in ids if rule(j1[i],j2[i]))
            lo,hi=wilson(valid,len(ids))
            hit,n=res[st][tier][0],res[st][tier][1]
            est=hit/n*valid/len(ids)
            print(f"  {tier}/{st}: judged {len(ids)}, valid {valid} (precision {valid/len(ids):.1%} [{lo:.1%}-{hi:.1%}]); raw match {hit}/{n}={hit/n:.1%}; ESTIMATED valid match rate={est:.1%} (CI using precision bounds: {hit/n*lo:.1%}-{hit/n*hi:.1%})")
