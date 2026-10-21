"""Frozen signature function (designed on DEV only). T1 = fine template, T2 = coarse class."""
import re
ENV_ECHO=re.compile(r"^[A-Z][A-Z0-9_]{2,}:\s")
NOISE=re.compile(r"^(##\[|level=|\d{4}-\d\d-\d\dT|\[?(TRACE|DEBUG|INFO)\b)|^(with|env|run):\s*$",re.I)
ERRRX=re.compile(r"(?i)\b(error|errors|failed|failure|fatal|panic|exception|denied|unauthorized|forbidden|not found|timeout|timed out|unable to|could not|cannot|bad credentials|invalid|refused|http \d{3})\b|\b[A-Z]{1,3}\d{3,5}\b")
GENERIC=re.compile(r"(?i)^(##\[error\])?\s*(process completed with exit code \d+\.?|.*\bexited? (with )?code \d+\.?)\s*$")
def primary(excerpt,errors):
    for l in reversed([x.strip() for x in excerpt if x.strip()]):
        if NOISE.search(l) or ENV_ECHO.search(l): continue
        if ERRRX.search(l) and not GENERIC.search(l): return l
    for e in errors:
        e=e.replace("##[error]","").strip()
        if e and not GENERIC.search(e): return e
    return None
URL=re.compile(r"https?://([^/\s)\"']+)[^\s)\"']*")
PATH=re.compile(r"(?:[A-Za-z]:)?(?:\.{0,2}/)?(?:[\w@.\-+~]+/)+[\w@.\-+~]*(?::\d+(?::\d+)?|\(\d+(?:,\d+)?\))?")
HEX=re.compile(r"\b[0-9a-f]{7,}\b"); VER=re.compile(r"\bv?\d+(?:\.\d+){1,3}(?:[-+][\w.]+)?\b"); NUM=re.compile(r"\b\d+(?:\.\d+)?(?:[a-z]{1,3})?\b")
QUOTE=re.compile(r"'[^']*'|\"[^\"]*\"|`[^`]*`")
def normalize(s):
    s=s.replace("##[error]","").strip()
    s=URL.sub(lambda m:"url:"+m.group(1),s); s=QUOTE.sub("<q>",s); s=PATH.sub("<path>",s)
    s=HEX.sub("<hex>",s); s=VER.sub("<ver>",s); s=NUM.sub("<n>",s)
    return re.sub(r"\s+"," ",s).strip().lower()[:160]
ECO=[("terraform",r"terraform|tflint|\.tf\b"),("kube",r"helm|kubectl|kubernetes|kind "),("docker",r"docker|buildx|container"),
 ("rust",r"cargo|rustc|clippy|error\[e\d+\]"),("go",r"golangci|go build|go mod|go test|\.go:"),
 ("node",r"npm|pnpm|yarn|node_modules|tsc|eslint|prettier|\bts\d{4}\b"),("python",r"\bpip\b|\buv\b|pytest|pylint|mypy|python|\.py\b"),
 ("java",r"gradle|maven|mvn"),("dotnet",r"dotnet|msbuild"),("os-pkg",r"brew|apt-get|\bapt\b|choco"),
 ("github",r"actions/github-script|gh api|api\.github\.com|github-action|github\.com/rest"),("git",r"\bgit\b")]
def eco(step,job,excerpt):
    t=" ".join([step or "",job or ""]+[x for x in excerpt[-25:]]).lower()
    for n,rx in ECO:
        if re.search(rx,t): return n
    return "other"
CLS=[("http5xx",r"\b50[0-4]\b|bad gateway|service unavailable|server error|gateway time-?out"),
 ("auth",r"bad credentials|unauthorized|forbidden|permission denied|authentication|\b40[13]\b|not authorized|access denied"),
 ("rate-limit",r"rate limit|too many requests|\b429\b"),
 ("network",r"econn|etimedout|eai_again|could not resolve|connection (reset|refused|timed out)|network|tls|ssl|socket|stream error|httperror|download"),
 ("timeout",r"timed? ?out|deadline exceeded|exceeded the maximum"),
 ("not-found",r"not found|no such file|cannot find|could not find|does not exist|\b404\b"),
 ("resource",r"no space left|out of memory|\boom\b|killed|exit code 137"),
 ("type-error",r"\bts\d{4}\b|type error|incompatible types|mypy|cannot be named"),
 ("compile-error",r"could not compile|compilation|syntax error|cannot find symbol|undefined reference|error\[e\d+\]|\be0\d{3}\b"),
 ("lint-format",r"lint|format|prettier|style issues|gofmt|ruff|pylint|eslint|clippy|stylelint|\bgci\b|misspell"),
 ("dependency",r"resolv|dependency|no solution found|version conflict|lockfile|unsatisfied|no matching distribution|could not import"),
 ("policy",r"must have|label|milestone|required"),("assertion",r"assert|expected .* (got|but)")]
def cls(line):
    l=line.lower()
    for n,rx in CLS:
        if re.search(rx,l): return n
    return "other"
def sign(r):
    p=primary(r["excerpt"],r["errors"])
    e=eco(r.get("step"),r.get("job"),r["excerpt"])
    if p is None: return dict(primary=None,eco=e,T1=None,T2=None)
    return dict(primary=p,eco=e,T1=(e,normalize(p)),T2=(e,cls(p)))
