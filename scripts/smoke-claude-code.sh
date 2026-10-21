#!/bin/bash
# Real-agent smoke test of the Claude Code adapter (spike S3).
# Needs: the `claude` CLI (tested with 2.1.302), git, python3, a built `opshero`.
# Cost: a few cents of API usage (three short sessions with the haiku model).
# Usage: OPSHERO_BIN=target/debug/opshero scripts/smoke-claude-code.sh
set -u
BIN=$(realpath "${OPSHERO_BIN:-target/debug/opshero}")
W=$(mktemp -d); L=$W.logs; mkdir -p "$L"; cd "$W" || exit 1
git init -q -b main && git config user.email t@e.c && git config user.name t
printf '#!/bin/sh\necho "all tests passed"\nexit 0\n' > mytests.sh; echo "code v1" > code.txt
git add -A && git commit -qm init
"$BIN" init >/dev/null
python3 - <<'PY'
p=".opshero/policy.toml"; s=open(p).read(); open(p,"w").write(s.replace('["pytest", "*"]','["sh", "mytests.sh"]'))
PY
# A raw-payload logger registered next to our hooks, writing OUTSIDE the repository.
mkdir -p .claude
printf '#!/bin/sh\ncat >> "$PAYLOAD_LOG"; printf "\\n" >> "$PAYLOAD_LOG"; exit 0\n' > .claude/log_hook.sh; chmod +x .claude/log_hook.sh
cat > .claude/settings.json <<'J'
{"hooks":{"PostToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"./.claude/log_hook.sh"}]}],
"PostToolUseFailure":[{"matcher":"Bash","hooks":[{"type":"command","command":"./.claude/log_hook.sh"}]}],
"Stop":[{"hooks":[{"type":"command","command":"./.claude/log_hook.sh"}]}]}}
J
"$BIN" hook install --scope local --yes >/dev/null
export PAYLOAD_LOG="$L/payloads.jsonl"
ask() { local n=$1 p=$2; shift 2
  timeout 170 claude -p "$p" --model haiku --max-turns 8 --output-format json "$@" </dev/null >"$L/$n.out.json" 2>"$L/$n.err.txt"
  echo "[$n] claude exit=$?"; }
echo "=== A: test passes (expect PASS)"
ask A "Run exactly this shell command and nothing else: sh mytests.sh   Then reply with the single word DONE." --allowedTools "Bash(sh mytests.sh)"
"$BIN" verify; echo "verify exit=$?"
echo "=== B: test fails (expect FAIL, exit code 1)"
printf '#!/bin/sh\necho "1 test FAILED"\nexit 1\n' > mytests.sh
ask B "Run exactly this shell command and nothing else: sh mytests.sh   Then reply with the single word DONE." --allowedTools "Bash(sh mytests.sh)"
"$BIN" verify; echo "verify exit=$?"
echo "=== C: code edited after the tests, Stop gate on (expect a blocked Stop, a re-run, final PASS)"
printf '#!/bin/sh\necho "all tests passed"\nexit 0\n' > mytests.sh
"$BIN" hook install --scope local --enforce-stop --yes >/dev/null
ask C "Do these steps in order: 1) run the shell command: sh mytests.sh  2) use the Edit tool (not Bash) to replace the text 'code v1' with 'code v2' in code.txt  3) reply with the single word DONE." --allowedTools "Bash(sh mytests.sh)" --permission-mode acceptEdits
"$BIN" verify; echo "verify exit=$?"
echo "--- event sequence (look for a Stop followed by a second PostToolUse and a Stop with stop_hook_active=true):"
python3 - "$L/payloads.jsonl" <<'PY'
import json,sys
for i,l in enumerate(x for x in open(sys.argv[1]) if x.strip()):
    d=json.loads(l); e=d["hook_event_name"]
    print(i,e,{"Stop":f"stop_hook_active={d.get('stop_hook_active')}","PostToolUseFailure":repr(d.get("error","")[:30])}.get(e,repr(d.get("tool_input",{}).get("command"))))
PY
echo "workdir: $W   logs: $L"
