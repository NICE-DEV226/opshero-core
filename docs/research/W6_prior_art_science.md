# W6 - Prior art and science: failure diagnosis and reproducible environments (2026-10-19)

Method note: web search is thin/US-only; many figures come from search-result summaries plus abstract fetches, NOT full-paper reads. Sample sizes are given where seen; "unverified" = not confirmed against primary text. Labels: FACT (primary source), REPORTED (secondary/vendor), INFERENCE (mine).

## Ranked key findings

1. **Specialized agent-facing interfaces matter, but mostly for editing/search, not "diagnosis context".** SWE-agent's ACI ablation (SWE-bench Lite, GPT-4 Turbo): shell-only 11.0% -> 18.0% with ACI; removing the edit command 10.3%, removing search 15.7% (FACT paper; numbers via secondary summary, 2024-era model). Evidence FOR structured tooling, but old models.
2. **Newer evidence leans AGAINST specialized tool catalogs.** "Is Bash All You Need?" (arXiv 2609.11999): bash alone beat typed tools by 21.8-24.5 pts (TheAgentCompany) and 4.8-7.4 pts (APEX-Agents), 19-72% fewer tokens; adding typed tools to bash gave no detectable pooled gain. Only 2 benchmarks, 2 models (Opus-4.8, GPT-5.5), sample sizes not seen; not a debugging benchmark (FACT preprint, INFERENCE on relevance). SWE-bench "bash-only" mini-SWE-agent tracks frontier within a few points (REPORTED, swebench.com/bash-only).
3. **But agents that build their own tools win: Live-SWE-agent** (bash start + self-created tools) 77.4% SWE-bench Verified (v3; v1 said 75.4%); removing tool creation 76.0 -> 62.0 on a 50-problem subset (ablation via alphaxiv summary, tiny n). Reading: tools help when targeted to the task at hand and created on demand; static generic catalogs do not (INFERENCE).
4. **Extra static context can hurt.** AGENTS.md study (Gloaguen et al., ICLR 2026, arXiv 2602.11988): context files lowered success vs none and raised cost >20%; agents over-explore when instructed (AGENTBENCH, 138 Python issues, 4 models; FACT abstract-level). Strong warning against "dump more context" tools: context must be verified-relevant, not generic.
5. **CI-failure repair is hard and uneven; "easy" classes are tool-enforced.** CI-Repair-Bench (567 failures, 103 repos, 12 error types): best LLM 18.9% repair; formatting/lint work best; env/dependency/config hardest (FACT abstract). CI-Bench (ICSE'26 demo, 100 BugSwarm failures): up to 32% / 36% / 13% for Agentless / SWE-Agent / AutoCodeRover (REPORTED). GradleFixer: domain-specific tools lifted Android build repair to 81.4% pass@1 on 1,019 failures (arXiv 2510.08640; authors' own benchmark, REPORTED) - the best FOR-evidence for domain tools, in a narrow ecosystem with a closed oracle.
6. **Log reduction is a real, cheap, measurable win but trivially clonable.** LogSieve: -42% lines / -40% tokens, cosine 0.93, 20 Android GitHub Actions repos (FACT abstract). LogSage (ByteDance): preprocessing + structured prompting, 98% RCA precision on 367 cases / 76 repos, +12% over their own naive LLM baseline; >88% end-to-end industrially (authors' own baselines, REPORTED). Neither tests against an agent with grep/shell - exactly the comparison your A/B null result suggests matters (INFERENCE).
7. **Bisect/delta-debugging are solved classics; LLM add-ons are modest.** LLM-in-the-loop git bisect: success 74.2% -> 80.6%, up to 2x faster, mostly on flaky/non-monotonic cases (arXiv 2511.18854, FACT abstract, dataset size not seen). LLMBisect: 91% bug-inducing-commit accuracy (REPORTED, snippet). I found NO benchmark of agents doing bisect/ddmin-style localization (gap).
8. **Flakiness is the best-documented deterministic-signal problem, but numbers need primary verification.** Google: ~16% of tests show some flakiness; ~84% of post-submit pass->fail transitions flaky; 1.5% of runs (REPORTED, aggregator blogs citing Micco 2016; I could not retrieve the primary post body - UNVERIFIED). Google's own post admits they do not track how often flakiness masks bugs (FACT, blog comments). No Meta source found.
9. **Reproducible envs: strong tooling, no outcome evidence.** nixpkgs: 91% packages bitwise reproducible; 99.94% of 14k packages rebuilt from a 6-year-old revision (REPORTED via nixos.org/research and citing paper). I found no empirical study that Nix/devcontainers reduce "works on my machine" failures (gap). Adoption is real: mise 35k stars, devbox 12k, flox 4k, nixpkgs 26k (FACT gh api, all pushed within 5 days).
10. **Standards exist but are young.** OTel CI/CD semconv: status "Development" (one page says RC - inconsistent), spans/metrics/logs, since v1.27.0 (REPORTED). Develocity now ships an MCP server (failure grouping, stack traces, flaky-vs-consistent skills in 2026.1) - incumbent vendor moving into agent-facing diagnosis (vendor docs, FACT of existence, not of efficacy).

## Evidence table

| Claim | Source | Label |
|---|---|---|
| ACI ablation 11->18% | https://arxiv.org/pdf/2405.15793 | FACT (numbers via secondary) |
| Bash beats typed tools | https://arxiv.org/abs/2609.11999 | FACT preprint |
| Bash-only SWE-bench leaderboard | https://www.swebench.com/bash-only.html | REPORTED |
| Live-SWE-agent | https://arxiv.org/abs/2511.13646 | FACT; ablation REPORTED |
| AGENTS.md hurts | https://arxiv.org/abs/2602.11988 | FACT |
| CI-Repair-Bench | https://arxiv.org/abs/2604.27148 | FACT |
| CI-Bench | https://conf.researchr.org/details/icse-2026/icse-2026-demonstrations/9/CI-Bench-A-Framework-for-Evaluating-Large-Language-Model-Tools-on-CI-Failures | REPORTED |
| GradleFixer / AndroidBuildBench | https://arxiv.org/pdf/2510.08640 | REPORTED (authors' bench) |
| LogSage | https://arxiv.org/abs/2506.03691 | FACT; own baselines |
| LogSieve | https://arxiv.org/abs/2601.20148 | FACT |
| LLM git bisect | https://arxiv.org/abs/2511.18854 | FACT |
| LLMBisect | https://arxiv.org/pdf/2510.26086 | REPORTED |
| Google flaky post | https://testing.googleblog.com/2016/05/flaky-tests-at-google-and-how-we.html | FACT existence; % UNVERIFIED |
| Nix reproducibility | https://nixos.org/research | REPORTED |
| OTel CI/CD semconv | https://opentelemetry.io/docs/specs/semconv/cicd/ | FACT |
| Develocity MCP | https://docs.gradle.com/develocity/2026.1/integrations/mcp-servers/ | FACT (vendor) |
| Repo stats | https://github.com/jdx/mise , /jetify-com/devbox , /flox/flox , /SWE-agent/mini-swe-agent , /BugSwarm/bugswarm (38 stars, NOASSERTION license) | FACT gh api |

## Strongest FOR vs AGAINST "structured deterministic context beats raw exploration"

FOR: (a) SWE-agent ACI +64% relative over shell-only; (b) GradleFixer domain tools 81.4% in a closed ecosystem; (c) Live-SWE-agent: removing tool creation costs ~14 pts; (d) AGENTS.md study shows unstructured exploration-inducing context costs >20% more and hurts, implying precision matters; (e) logs: 40% token cut with fidelity.
AGAINST: (a) Bash-only ~ frontier on SWE-bench; (b) "Is Bash All You Need": typed tools add nothing, cost more; (c) models improved faster than scaffolds; (d) your own null A/B on easy cases; (e) no paper I found shows a deterministic diagnosis tool beating a strong agent with shell on CI failures.
Synthesis (INFERENCE): the benefit concentrates where (1) the agent cannot cheaply reproduce the information (remote CI state, other-machine env, historical runs, cross-run statistics), (2) the output is verified/small, and (3) the task is hard. Where info is in the repo or the local shell, the agent re-derives it.

## Reusable rather than rebuilt
- git bisect/`git bisect run`, ddmin implementations (Perses/C-Reduce-style) - solved, wrap don't rewrite.
- OTel CI/CD semconv (Development status; expect churn) for run/job/step schema; Bazel BES for build events; JUnit XML / SARIF for test and lint results.
- mise/devbox/flox/Nix/devcontainers for env capture; nixpkgs reproducibility evidence.
- BugSwarm (38 stars, weak license), CI-Repair-Bench, CI-Bench, LogSage's 367-case set (promised open) for evals; mini-swe-agent (MIT, 8.6k stars) as the neutral harness for A/B.
- Develocity MCP is an incumbent for Gradle/Maven; do not compete head-on there.

## Implications for OpsHero
1. Do not sell "summarize/diagnose the log": LogSage/LogSieve-class work is published, clonable, and unproven against grep-capable agents.
2. Value must come from data an agent cannot get by exploring: cross-run history (flaky probability, first-bad-commit across runs), environment deltas between passing/failing runners, remote artifacts. This matches the failed "since last green" test being null only on easy cases (INFERENCE: re-test on env/dependency/config failures, which CI-Repair-Bench shows are the hard ones).
3. Design as shell-native CLI emitting small verified JSON (agents win with bash), not a large tool catalog; allow agent-synthesized composition.
4. Evaluate with a bash-only baseline on mini-swe-agent; claim only deltas against it, with n >= 100 and cost/tokens reported.

## Open questions
- Does any structured diagnosis beat bash-only agents on env/dependency CI failures (the hard 12-type tail)?
- Real flaky rate/cost in target teams (primary Google/Meta numbers unretrieved).
- Do Nix/devcontainer environments measurably cut CI-vs-local divergence?
- Agent bisect benchmark does not exist.

## Three product ideas with cheap falsifiable tests
1. **Env-delta oracle**: `opshero envdiff` compares failing CI run vs local/last-green (toolchain, lockfile hash, runner image, env vars) as JSON. Test: take 50 env/dependency failures from CI-Repair-Bench/BugSwarm; mini-swe-agent bash-only vs +envdiff; falsified if resolve-rate gain < 10 pts or token savings < 20%.
2. **Flake/first-bad ledger**: cross-run store that returns P(flaky) and first-bad commit for a failing test (history agents cannot see). Test: on 20 repos' 90-day Actions history, label by re-run; falsified if precision < 90% or agents with `gh run list` + shell reach the same answer in > 70% of cases at similar cost.
3. **Bisect-as-a-tool with verified repro**: `opshero bisect --cmd` wraps git bisect run + ddmin, returns culprit commit and minimal repro, robust to flakiness (re-run voting). Test: 30 historical regressions; falsified if bash-only agent finds the same culprit within 1.5x tokens in > 60% of cases.
