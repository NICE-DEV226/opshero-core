# Test 1 results (2026-10-19) — cross-repo failure-signature commons
Corpus: 429 failed jobs, 43 public repos (popular OSS), split by repo DEV 203 / TEST 226. Hashes in FREEZE.txt. Protocol: PROTOCOL.md (amendments 1-4 logged before data was read).
PRE-REGISTERED primary metric (valid cross-repo match, NON-TEST stratum, TEST split, n=133): fine signature T1 raw 3.0% (4/133); coarse T2 raw 65.4% but judged precision 2.9% (1/34) -> est. valid 1.9% (CI up to 9.8%). Threshold 15% -> NOT SUPPORTED. Judges: 2 blind LLM judges, 97% exact agreement (68/70).
EXPLORATORY (not pre-registered): fuzzy token-overlap nearest neighbour, top-50 pairs judged by 2 blind judges (92% agreement): same-cause-class valid 21/133 = 15.8% [10.6-22.9] (LOWER BOUND, only top-50 judged); strict same-fix 5/133 = 3.8% [1.6-8.5].
Valid families: PR changelog/label gates (repo-specific policy), pre-commit auto-format diffs, TS type errors, transient Go network errors (re-run), one shared vuln (GO-2026-6617), one tool rule (hawk).
Good-Turing coverage T1 = 0.65 (about 35% of failures are new fine signatures); top-5 signatures = 15% of logs (long tail).
Within-repo recurrence T1 = 63% (inflated by chronically red workflows) vs cross-repo 7-16% (unjudged precision).
Limits: popular-repo bias; n small; LLM judges same family; signature design is one regex-based attempt; 62% of failures are test failures (repo-specific by nature); one time window.
