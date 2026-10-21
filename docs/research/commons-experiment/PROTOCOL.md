# Test 1 — Can a cross-repo "failure signature commons" exist?  (PRE-REGISTERED 2026-10-19, before any data is read)

Question: for real CI failures in public repos, does a deterministic signature computed from one repo's failure
match failures in OTHER repos with the same root cause / remediation, at a useful rate?

Population: failed GitHub Actions jobs, public repos, created >= 2026-08-15 (source: earlier step-level sample, 47 popular repos).
  Known bias: popular, well-maintained OSS. Results may NOT transfer to small teams/private repos.
Unit: 1 failed job per run (first failed job), max 6 runs per repo, to avoid matrix duplicates and repo dominance.
Excerpt: lines of the failing step before the first `##[error]`, last 60 lines + the error line(s).
Split: by REPOSITORY, seeded random 50/50 -> DEV and TEST. Signature code is written using DEV only, then FROZEN
  (sha256 recorded) before TEST is examined. TEST is run once.
Strata: (a) NON-TEST failures = failing step is not a test step (setup/deps/build/lint/action/infra...),
  (b) TEST failures. Test failures are expected to be repo-specific, so the PRIMARY metric is on stratum (a);
  stratum (b) and overall are reported for honesty.
Signature tiers (frozen later): T1 = normalized template of primary error line (+ecosystem); T2 = coarse (ecosystem + error class).
Match definition: a TEST log "matches" if >=1 DEV-split log from a DIFFERENT repo has the same signature.
Validity of a match: 2 blind LLM judges + my spot check of 20 pairs. Rubric: same root cause AND same remediation advice would apply
  (L1 strict = same cause & same fix; L2 = same cause, different specifics; L0 = different). Valid = majority >= L2; strict = majority L1.
PRIMARY METRIC: valid cross-repo match rate on NON-TEST TEST-split logs (Wilson 95% CI).
  Decision (pre-committed): >=30% => commons plausible; 15-30% => ambiguous (needs larger corpus; use Good-Turing coverage);
  <15% => commons not supported at this scale.
SECONDARY: Good-Turing coverage (1 - n1/N) of signatures on ALL logs, per stratum (extrapolates how much a bigger corpus would cover);
  cluster-size concentration (is it head-heavy, i.e. easy to clone, or long-tail?); baselines (step name only; exact raw error line).
Limitations to be reported: popular-repo bias, LLM judges, small n, 90-day log retention, one time window, regex-based extraction.

## AMENDMENT 1 (2026-10-19, made BEFORE any log excerpt was read)
Cap per repo raised from 6 to 12 runs, because the 47-repo pool would otherwise give < 300 logs. Original protocol hash: 37e7bab8e2da1cf7efbb7a46b630cc593896b82ac6c8dc65ad8bb9a3f3f57c14.

## AMENDMENT 2 (2026-10-19, still BEFORE any log excerpt was read)
The earlier step-level sample file was lost when the session's scratchpad was reset. The pool is therefore RE-SAMPLED with the same method:
47 repos (pandas, numpy, scipy, scikit-learn, matplotlib, pytorch, transformers, next.js, react, node, deno, tokio, home-assistant, grafana, prometheus,
compose, moby, airflow, superset, ansible, django, flask, requests, pip, pytest, ruff, uv, playwright, cypress, storybook, vite, webpack, eslint, laravel,
dotnet/runtime, julia, terraform-provider-aws, otel-python, otel-collector-contrib, kind, helm, argo-cd, istio, cilium, prisma, supabase, vllm, ray, mlflow, langchain),
runs with conclusion=failure, created >= 2026-08-15, events push|pull_request|schedule|workflow_dispatch, up to 100 per repo then random 12 per repo.
TEST-STEP definition (frozen now): failing step name matches regex  \b(test|tests|testing|pytest|jest|vitest|mocha|spec|specs|e2e|integration|unit|tox|nox)\b  (case-insensitive).
Everything else = NON-TEST stratum.

## AMENDMENT 3 (2026-10-19, bug fix, still BEFORE any excerpt content was read)
First corpus attempt gave only 62/554 logs: `gh api .../jobs/ID/logs` refuses output containing ANSI escape sequences unless
`--allow-escape-sequences` is passed, which would have silently biased the sample toward uncolored logs. Fixed by passing the flag and stripping
all ANSI/control sequences in code. Raw logs are treated as untrusted data (parsed only, never printed raw). No change to sampling, strata or metrics.

## AMENDMENT 4 (2026-10-19, still BEFORE any excerpt content was read)
For speed, only the LAST 512 KiB of each job log is downloaded (HTTP Range request on the pre-signed log URL; the GitHub token is sent only to api.github.com, never to the storage host).
The sample of (repo, run) pairs is REUSED from the first attempt (554 runs, 47 repos, same seed); only identifiers are reused, no content was read.
Runs whose first `##[error]` marker falls outside the tail are recorded as `error_outside_tail` and reported (bias: very long post-failure steps).
Excerpt window (60 lines before the first error marker, inside the failing step) is unchanged.

## FREEZE (2026-10-19) — signature code and evaluation script frozen BEFORE the TEST split was examined
See FREEZE.txt for sha256 of signature.py, evaluate.py, corpus.json, dev_repos.json. Design notes: primary error line = last informative non-generic line of the excerpt;
generic "exit code N" lines yield NO signature (counted as unsignable, never as a match); T1 = (ecosystem, normalized template), T2 = (ecosystem, error class);
baselines B1 = normalized failing step name, B2 = exact raw primary line. TEST is evaluated exactly once with evaluate.py.
