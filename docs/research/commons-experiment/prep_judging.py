import json, random, collections
ps=json.load(open("pairs_to_judge.json"))
print("pairs",len(ps),collections.Counter((p["tier"],p["stratum"]) for p in ps))
random.Random(11).shuffle(ps)
key={}; blind=[]
for i,p in enumerate(ps):
    key[i]={"tier":p["tier"],"stratum":p["stratum"],"orig_id":p["id"]}
    def fmt(x): return {"repo":x["repo"],"failing_step":x["step"],"last_log_lines":[l[:170] for l in x["excerpt"][-8:]],"error_annotation":[e[:170] for e in x["err"]]}
    blind.append({"id":i,"A":fmt(p["a"]),"B":fmt(p["b"])})
json.dump(key,open("judge_key_PRIVATE.json","w"))
half=len(blind)//2
for n,b in (("1",blind[:half]),("2",blind[half:])): json.dump(b,open(f"judge_batch_{n}.json","w"))
print("batches",half,len(blind)-half)
