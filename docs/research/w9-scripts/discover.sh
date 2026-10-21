#!/bin/bash
# Discover candidate agent PRs, weekly slices, created 2026-07-06..2026-09-13. Output: discover/<type>_<start>.json
cd "$(dirname "$0")"; mkdir -p discover
declare -A Q=( [copilot]='author:app/copilot-swe-agent' [devin]='author:app/devin-ai-integration' [cursor]='author:app/cursor' [claudebot]='author:app/claude' [claudecode]='"Generated with [Claude Code]" in:body' )
starts=(2026-07-06 2026-07-13 2026-07-20 2026-07-27 2026-08-03 2026-08-10 2026-08-17 2026-08-24 2026-08-31 2026-09-07)
ends=(2026-07-12 2026-07-19 2026-07-26 2026-08-02 2026-08-09 2026-08-16 2026-08-23 2026-08-30 2026-09-06 2026-09-13)
for i in "${!starts[@]}"; do for t in copilot devin cursor claudebot claudecode; do
  f=discover/${t}_${starts[$i]}.json; [ -s "$f" ] && continue
  for try in 1 2 3 4; do
    gh search prs ${Q[$t]} --created ${starts[$i]}..${ends[$i]} --limit 40 --json repository,number,author,createdAt,state > "$f.tmp" 2>err.txt && { mv "$f.tmp" "$f"; break; }
    echo "retry $t ${starts[$i]}: $(head -c 80 err.txt)"; sleep 90
  done; sleep 8
done; done
echo done
