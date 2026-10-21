#!/usr/bin/env bash
# Pilot: headless Claude Code sessions with the opshero hooks and journal enabled, each in a
# disposable repository. It measures the MECHANISM (does the Stop gate block what it should,
# and what friction does it create?), NOT demand: the sessions are scripted, not real work.
#
# Cost: real API usage. Each session is capped by --max-budget-usd (default 0.25). The Bash tool
# is limited to python3, git and a few read-only utilities, inside throwaway directories.
#
# Usage: PILOT_OUT=/some/dir [ONLY=t1] [REPS=3] [PAR=4] [BUDGET=0.25] [OPSHERO_BIN=target/release/opshero] scripts/pilot-dogfood.sh
# Then:  python3 scripts/pilot_analyze.py $PILOT_OUT
set -u
BIN=$(realpath "${OPSHERO_BIN:-target/release/opshero}")
OUT=${PILOT_OUT:?set PILOT_OUT to an empty or new directory}
REPS=${REPS:-3}; PAR=${PAR:-4}; BUDGET=${BUDGET:-0.25}
mkdir -p "$OUT/sessions"

make_base() { # $1 = dir  $2 = buggy? (1/0)  $3 = gitignore? (1/0)
  local d=$1; mkdir -p "$d"; cd "$d" || exit 1
  git init -q -b main . && git config user.email t@e.c && git config user.name t
  local sub='a - b'; [ "$2" = 1 ] && sub='a + b'
  cat > calc.py <<PY
def add(a, b):
    return a + b


def sub(a, b):
    return $sub


def mul(a, b):
    return a * b


def div(a, b):
    return a / b
PY
  cat > test_calc.py <<'PY'
import unittest
import calc


class CalcTest(unittest.TestCase):
    def test_add(self):
        self.assertEqual(calc.add(2, 3), 5)

    def test_sub(self):
        self.assertEqual(calc.sub(5, 3), 2)

    def test_mul(self):
        self.assertEqual(calc.mul(4, 3), 12)

    def test_div(self):
        self.assertEqual(calc.div(6, 3), 2)

    def test_div_by_zero(self):
        with self.assertRaises(ZeroDivisionError):
            calc.div(1, 0)


if __name__ == "__main__":
    unittest.main()
PY
  printf '# calc\n\nTiny calculator module.\n' > README.md
  printf '# Project notes\n\nRun the tests with: python3 -m unittest discover -q\n' > CLAUDE.md
  [ "$3" = 1 ] && printf '__pycache__/\n' > .gitignore
  git add -A && git commit -qm init
}

# id|base|prompt
TASKS=(
"t1_fix|bug|The tests are failing. Find the bug in calc.py and fix it."
"t2_feature|nogi|Add a function pow_(a, b) to calc.py that returns a to the power b, and add a test for it in test_calc.py."
"t3_refactor|ok|Rename the function mul to multiply everywhere (calc.py and test_calc.py)."
"t4_docs|ok|Add a short Usage section to README.md that shows how to call add()."
"t5_question|ok|In two sentences, explain what div() does when the divisor is zero. Do not change any files."
"t6_pipe|ok|Run the tests and show me only the last 3 lines of the output. Do not change any files."
"t7_edit_after|bug|Fix the bug in sub() in calc.py and run the tests. Then add a one-line docstring to add(). Finish immediately after adding the docstring and do not run the tests again."
"t8_comment_after|ok|Run the tests, then add the comment line '# reviewed' at the top of calc.py, and stop. Do not run the tests again."
)

make_base "$OUT/base_ok" 0 1; make_base "$OUT/base_bug" 1 1; make_base "$OUT/base_nogi" 0 0

run_one() {
  local id=$1 base=$2 prompt=$3
  local S="$OUT/sessions/$id"; mkdir -p "$S"; cp -r "$OUT/base_$base" "$S/repo"; cd "$S/repo" || return
  "$BIN" init >/dev/null 2>&1
  printf 'version = 1\nid = "pilot"\n[[require]]\nid = "tests"\ncheck = "evidence.fresh"\ncommand = ["python3", "-m", "unittest", "*"]\npaths = ["*.py"]\n' > .opshero/policy.toml
  "$BIN" hook install --scope local --enforce-stop --yes >/dev/null 2>&1
  "$BIN" journal enable >/dev/null 2>&1
  timeout 300 claude -p "$prompt" --model haiku --max-turns 15 --max-budget-usd "$BUDGET" \
    --output-format stream-json --verbose --permission-mode acceptEdits \
    --allowedTools "Bash(python3:*)" "Bash(python:*)" "Bash(git:*)" "Bash(ls:*)" "Bash(cat:*)" "Bash(grep:*)" "Bash(head:*)" "Bash(tail:*)" "Bash(wc:*)" \
    </dev/null >"$S/stream.jsonl" 2>"$S/err.txt"
  echo "claude_exit=$?" > "$S/exit.txt"
  cp .git/opshero/journal.jsonl "$S/journal.jsonl" 2>/dev/null || : > "$S/journal.jsonl"
  "$BIN" verify --format json > "$S/verify.json" 2>/dev/null; echo "verify_exit=$?" >> "$S/exit.txt"
  git status --porcelain > "$S/status.txt" 2>/dev/null
  echo "done $id"
}
export -f run_one; export OUT BIN BUDGET

JOBS=()
for rep in $(seq "$REPS"); do for t in "${TASKS[@]}"; do
  IFS='|' read -r tid base prompt <<<"$t"
  if [ -n "${ONLY:-}" ] && [[ $tid != $ONLY* ]]; then continue; fi
  JOBS+=("${tid}_r${rep}|$base|$prompt")
done; done
printf '%s\n' "${JOBS[@]}" | xargs -P "$PAR" -I{} bash -c 'IFS="|" read -r id base prompt <<<"{}"; run_one "$id" "$base" "$prompt"'
echo "all sessions finished: $OUT/sessions"
