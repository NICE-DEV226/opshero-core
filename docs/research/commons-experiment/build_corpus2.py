import json, subprocess, re, collections, sys, os, concurrent.futures as cf
TAIL=524288
TOKEN=subprocess.run(["gh","auth","token"],capture_output=True,text=True).stdout.strip()
sel=[(r["repo"],r["run"]) for r in json.load(open("selection_from_attempt1.json"))][:int(os.environ.get("LIMIT","99999"))]
TS=re.compile(r"^﻿?\d{4}-\d\d-\d\dT[\d:.]+Z ")
ANSI=re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b[@-Z\\-_]|[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]")
def gh_json(path):
    r=subprocess.run(["gh","api","-X","GET",path],capture_output=True)
    try: return json.loads(r.stdout) if r.returncode==0 else None
    except Exception: return None
def tail_logs(repo,jid):
    h=subprocess.run(["curl","-s","-m","30","-o","/dev/null","-D","-","-H",f"Authorization: Bearer {TOKEN}","-H","Accept: application/vnd.github+json",
        f"https://api.github.com/repos/{repo}/actions/jobs/{jid}/logs"],capture_output=True,text=True).stdout
    m=re.search(r"^location:\s*(\S+)",h,re.I|re.M)
    if not m: return None
    url=m.group(1)
    p=subprocess.run(["curl","-s","-m","30","-r","0-0","-D","-","-o","/dev/null",url],capture_output=True,text=True).stdout
    t=re.search(r"content-range:\s*bytes\s+\d+-\d+/(\d+)",p,re.I)
    total=int(t.group(1)) if t else None
    rng=f"{max(0,total-TAIL)}-" if total else "0-"
    b=subprocess.run(["curl","-s","-m","60","-r",rng,url],capture_output=True).stdout
    return b.decode("utf8","replace"),(total is not None and total>TAIL)
def work(t):
    repo,rid=t
    d=gh_json(f"repos/{repo}/actions/runs/{rid}/jobs?per_page=100")
    if not d: return {"repo":repo,"run":rid,"status":"jobs_err"}
    fj=[j for j in d["jobs"] if j["conclusion"]=="failure"]
    if not fj: return {"repo":repo,"run":rid,"status":"no_failed_job"}
    j=fj[0]; fs=[s for s in j["steps"] if s["conclusion"]=="failure"]
    step=fs[0]["name"] if fs else None
    res=tail_logs(repo,j["id"])
    if not res: return {"repo":repo,"run":rid,"job_id":j["id"],"status":"logs_err"}
    txt,truncated=res
    lines=[ANSI.sub("",TS.sub("",l)) for l in txt.splitlines()]
    ei=next((i for i,l in enumerate(lines) if "##[error]" in l),None)
    if ei is None: return {"repo":repo,"run":rid,"job_id":j["id"],"status":"error_outside_tail" if truncated else "no_error_marker"}
    start=max((i for i in range(ei) if lines[i].startswith("##[group]Run ")),default=max(0,ei-60))
    exc=lines[max(start+1,ei-60):ei]
    errs=[]; k=ei
    while k<len(lines) and "##[error]" in lines[k] and len(errs)<4: errs.append(lines[k]); k+=1
    return {"repo":repo,"run":rid,"job_id":j["id"],"job":j["name"],"step":step,"status":"ok","excerpt":exc,"errors":errs}
out=[]
with cf.ThreadPoolExecutor(16) as ex:
    for i,r in enumerate(ex.map(work,sel)):
        out.append(r)
        if i%50==0: print(i,flush=True)
json.dump(out,open("corpus_raw.json","w"))
print(collections.Counter(r["status"] for r in out),flush=True)
