#!/usr/bin/env python3
"""Analyse the pilot sessions (scripts/pilot-dogfood.sh).

Ground truth is computed from the agent's transcript, independently of opshero:
  changed      = the agent edited a file (Edit/Write tools, or a Bash command that writes)
  docs_only    = every edited file is Markdown
  verified     = a test command ran AFTER the last edit and its output says OK
Ideal gate decision for the session's first Stop:
  block  iff the content changed (not docs only) and was not verified afterwards
  allow  otherwise (nothing changed, docs only, or verified)
Then compare with what the journal says the gate did. Stdlib only.
"""
import json, math, os, re, sys

TEST_RE = re.compile(r"unittest|pytest|python3? +-m +test|test_\w+\.py")
PYWRITE_RE = re.compile(r"python3?\s+(-\s*<<|-c)")
WRITE_RE = re.compile(r"(?<![\d&])>>?\s*(?!/dev/null|&)\S|\bsed\s+-i|\btee\b|\bmv\b|\bcp\b|\brm\b")


def wilson(s, n, z=1.959964):
    if n == 0:
        return None
    p = s / n; d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return max(0.0, c - h), min(1.0, c + h)


def pct(s, n):
    if n == 0:
        return "n/a"
    lo, hi = wilson(s, n)
    return f"{s}/{n} = {100*s/n:.0f}% (95% CI {100*lo:.0f}-{100*hi:.0f}%)"


def transcript(path):
    """Return (steps, cost, turns, blocked). steps: what the agent did BEFORE the first Stop
    hook feedback (the gate's intervention), because what it does afterwards is the gate's effect,
    not the state the gate had to judge. blocked: whether such feedback was seen."""
    steps, calls, cost, turns, blocked = [], {}, None, None, False
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if e.get("type") == "result":
            cost = e.get("total_cost_usd"); turns = e.get("num_turns")
        msg = e.get("message")
        if not isinstance(msg, dict):
            continue
        content = msg.get("content")
        if isinstance(content, str):
            content = [{"type": "text", "text": content}]
        if not isinstance(content, list):
            continue
        if not blocked and e.get("type") == "user" and any(
            isinstance(b, dict) and str(b.get("text", "")).startswith("Stop hook feedback") for b in content
        ):
            blocked = True
        if blocked:
            continue
        for b in content:
            if not isinstance(b, dict):
                continue
            if b.get("type") == "tool_use":
                name, inp = b.get("name"), b.get("input") or {}
                st = {"id": b.get("id"), "tool": name, "kind": "other", "ok": None, "cmd": None, "paths": []}
                if name in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
                    st["kind"] = "edit"; st["paths"] = [inp.get("file_path", "")]
                elif name == "Bash":
                    cmd = inp.get("command", ""); st["cmd"] = cmd
                    if TEST_RE.search(cmd):
                        st["kind"] = "test"
                    if (WRITE_RE.search(cmd) or (PYWRITE_RE.search(cmd) and "open(" in cmd)) and st["kind"] != "test":
                        st["kind"] = "edit"; st["paths"] = ["<bash-write>"]
                steps.append(st); calls[st["id"]] = st
            elif b.get("type") == "tool_result":
                st = calls.get(b.get("tool_use_id"))
                if st is not None:
                    text = b.get("content")
                    if isinstance(text, list):
                        text = " ".join(x.get("text", "") for x in text if isinstance(x, dict))
                    text = text or ""
                    if st["kind"] == "test":
                        st["ok"] = (re.search(r"\bOK\b", text) is not None and "FAILED" not in text and "Error" not in text)
                    st["is_error"] = bool(b.get("is_error"))
    return steps, cost, turns, blocked


def analyse(sdir):
    steps, cost, turns, saw_feedback = transcript(os.path.join(sdir, "stream.jsonl"))
    # Ground truth at the moment of the first Stop, from the agent's own actions before any
    # intervention. When there was no intervention, git's final state is the reference.
    edits = [i for i, s_ in enumerate(steps) if s_["kind"] == "edit"]
    def code_like(s_):
        c = s_["cmd"] or ""
        return any(p.endswith(".py") for p in s_["paths"]) or (s_["paths"] == ["<bash-write>"] and ".py" in c) or (
            s_["paths"] == ["<bash-write>"] and ".md" not in c)
    status_files = []
    for l in open(os.path.join(sdir, "status.txt"), encoding="utf-8", errors="replace"):
        path = l[3:].strip()
        if path and not path.startswith((".claude", ".opshero", "__pycache__")):
            status_files.append(path)
    if saw_feedback:
        changed = bool(edits)
        docs_only = changed and not any(code_like(steps[i]) for i in edits)
    else:
        changed = bool(status_files) or bool(edits)
        docs_only = changed and (all(p.endswith(".md") for p in status_files) if status_files else not any(code_like(steps[i]) for i in edits))
    last_edit = max(edits) if edits else (0 if changed else None)
    verified = changed and last_edit is not None and any(s_["kind"] == "test" and s_["ok"] for s_ in steps[last_edit + 1:])
    ran_tests = any(s["kind"] == "test" for s in steps)
    piped = any(s["kind"] == "test" and re.search(r"\|", s["cmd"] or "") for s in steps)
    ideal_block = changed and not docs_only and not verified
    klass = ("unchanged" if not changed else "docs-only" if docs_only else "changed-verified" if verified else "changed-unverified")
    stops = []
    for l in open(os.path.join(sdir, "journal.jsonl"), encoding="utf-8"):
        try:
            e = json.loads(l)
        except ValueError:
            continue
        if e.get("kind") == "stop":
            stops.append(e)
    first = next((s for s in stops if not s.get("stop_active")), None)
    actual = None if first is None else first["decision"]
    try:
        final_verdict = json.load(open(os.path.join(sdir, "verify.json")))["verdict"]
    except Exception:
        final_verdict = "?"
    return dict(id=os.path.basename(sdir), klass=klass, ideal="block" if ideal_block else "allow",
                actual=actual or "no-stop", reasons=(first or {}).get("reasons", []), cost=cost, turns=turns,
                ran_tests=ran_tests, piped_test=piped, final=final_verdict, n_stops=len(stops))


def main(out):
    sess = sorted(os.path.join(out, "sessions", d) for d in os.listdir(os.path.join(out, "sessions")))
    rows = [analyse(s) for s in sess if os.path.exists(os.path.join(s, "stream.jsonl"))]
    print(f"{'session':<18}{'truth class':<20}{'ideal':<7}{'gate':<9}{'reasons':<22}{'final':<9}{'turns':<6}cost")
    for r in rows:
        print(f"{r['id']:<18}{r['klass']:<20}{r['ideal']:<7}{r['actual']:<9}{','.join(r['reasons']) or '-':<22}{r['final']:<9}{str(r['turns']):<6}{r['cost']}")
    n = len(rows)
    tp = sum(r["ideal"] == "block" and r["actual"] == "block" for r in rows)
    fn = sum(r["ideal"] == "block" and r["actual"] != "block" for r in rows)
    fp = sum(r["ideal"] == "allow" and r["actual"] == "block" for r in rows)
    tn = sum(r["ideal"] == "allow" and r["actual"] != "block" for r in rows)
    print(f"\nsessions: {n}   total cost: ${sum(r['cost'] or 0 for r in rows):.2f}")
    print(f"gate on sessions that SHOULD be blocked (changed, not verified): caught {pct(tp, tp + fn)}")
    print(f"gate on sessions that should be allowed: wrongly blocked {pct(fp, fp + tn)}")
    print("\nwrongly blocked, by truth class:")
    for k in ("unchanged", "docs-only", "changed-verified"):
        sub = [r for r in rows if r["klass"] == k]
        if sub:
            print(f"  {k:<18} {sum(r['actual'] == 'block' for r in sub)}/{len(sub)} blocked   reasons: {sorted({x for r in sub for x in r['reasons']})}")
    pipes = [r for r in rows if r["piped_test"]]
    print(f"\nsessions where the agent ran the tests through a pipeline: {len(pipes)} (blocked: {sum(r['actual'] == 'block' for r in pipes)})")
    print("sessions with no Stop event recorded (hook did not fire or session ended early):", sum(r["actual"] == "no-stop" for r in rows))
    json.dump(rows, open(os.path.join(out, "analysis.json"), "w"), indent=1)


if __name__ == "__main__":
    main(sys.argv[1])
