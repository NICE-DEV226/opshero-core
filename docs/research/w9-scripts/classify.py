import re
def category(pr):
    a = (pr.get("author") or {})
    login = (a.get("login") or ""); typ = a.get("__typename")
    hb = pr.get("headRefName") or ""; body = pr.get("bodyText") or ""
    msgs = " ".join((n["commit"]["message"] or "") for n in (pr.get("commits") or {}).get("nodes",[]))
    if login == "copilot-swe-agent" or login=="Copilot": return "copilot"
    if login.startswith("devin-ai-integration"): return "devin"
    if login in ("cursor","cursor[bot]"): return "cursor"
    if login in ("claude","claude[bot]"): return "claude_bot"
    if typ == "Bot": return "other_bot"
    if re.search(r"Co-Authored-By: Claude", msgs, re.I) or "Generated with [Claude Code]" in body or "Generated with Claude Code" in body: return "claude_code_human"
    if hb.startswith("codex/"): return "codex_branch_human"
    if hb.startswith("cursor/") or "Cursor Agent" in msgs: return "cursor_marker_human"
    return "human"
