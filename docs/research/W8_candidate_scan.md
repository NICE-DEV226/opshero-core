# W8 - Competitive / prior-art scan of 5 candidate product shapes (2026-10-19)

Labels: FACT = primary source (gh api, repo README, docs). REPORTED = secondary/vendor. INFERENCE = mine. Star/license/push data pulled via `gh api repos/<r>` on 2026-10-19 (URL pattern https://github.com/<r>). Search is US-only and thin: coverage is partial, and several items below are marked unverified.

## Ranked findings (most decision-relevant first)

1. **(2) Agent governance/sandboxing is the most crowded AND most funded space, with big-vendor absorption.** Anthropic's own sandbox-runtime (5.6k stars, Apache-2.0, pushed 2026-10-17), NVIDIA/OpenShell (16.7k), Microsoft agent-governance-toolkit (6.4k), Docker mcp-gateway (1.6k), agentgateway (5.4k), Snyk agent-scan (3.2k, requires a Snyk account/token) all exist. REPORTED (vendor blog, MintMCP, biased): Palo Alto bought Portkey (closed 2026-05-29) and Boomi bought Lunar.dev. The standalone OSS pieces by startups look stale: Invariant `invariant` last push 2026-01-12 (472 stars), Lasso mcp-gateway last push 2026-01-22 (396 stars) [FACT]. INFERENCE: a new entrant competes with platform vendors and free first-party runtimes; clone resistance is only trust/distribution, which incumbents hold.
2. **(5) Agent memory is crowded at the generic layer, and GitHub ships repo-scoped memory natively.** mem0 67k stars, claude-mem 102k, graphiti 32k, cognee 32.5k, letta 25k (all Apache-2.0, all pushed within days) [FACT]. Mem0 raised $24M (Oct 2025; split between seed/Series A disagrees across sources) [REPORTED]. GitHub Copilot Memory: public preview, repo-scoped, write-access contributors only, memories verified against cited code, 28-day expiry (28-day figure from VS Code docs only) [FACT-docs]. The gap is NOT "memory"; it is memory of *verified ops fixes*, and nobody I found does that with proof of verification (INFERENCE).
3. **(4) No healthy "error database" for CI/infra failures exists; the analogs show why that is hard.** Search found no maintained general OSS repository of CI error signatures; only research (SAP HANA signature matching; 2026 GitHub Actions flaky-build study: 3.2% of builds rerun, 67.73% of reruns flaky, 1,960 Java projects, so Java-only bias) and a small MCP "flakiness knowledge graph" server [REPORTED via search; arXiv 2602.02307]. Vendors keep this private: Datadog flaky management, Develocity Build Failure Analytics, Launchable gen-AI failure grouping [vendor docs/marketing]. Analogs are healthy because a *standard identifier* (CVE/package) exists: OSV (osv.dev 3.0k, schema 272 stars, Apache-2.0, active), Sigma (11.3k, active), semgrep-rules (1.3k, license NOASSERTION/non-OSS-like), Renovate (22.8k, AGPL). CI failures lack a stable key (log text varies by repo/version/env). INFERENCE: open lane but the cold-start is brutal and distribution is the moat, not code; the value is only realized with a canonical signature schema plus contributors.
4. **(3) Verification oracle: building blocks are OSS and mature; the agent-specific gap is real but unproven.** Mutation testing: stryker-js 3.2k, stryker-net 2.1k, mutmut 1.5k (all active) [FACT]. Test impact analysis is commercial (Launchable/CloudBees, OSS CLI smart-tests-cli only 27 stars; third-party listing says from ~US$12k/yr, unverified). Research: SWE-Mutation (ACL Findings 2026) reports LLM-written tests are superficial; best of 7 models reached 10.20% verification and 36.15% detection [REPORTED via search; not read in full]. INFERENCE: demand signal for "are these tests strong enough to trust an agent patch" is plausible; but the A/B null result on "what changed" suggests the agent already self-verifies on easy cases; the value is in hard/flaky cases only.
5. **(1) "What changed since it last worked" is mostly solved piecemeal by free, hugely popular tools.** atuin 32k (MIT), mise 35k (MIT), direnv 15.5k, devbox 12.4k (Apache-2.0), nix 17.9k, all pushed this month; `git bisect run` is built in; Develocity build scans/comparison is commercial (gradle/gradle 18.9k is the OSS build tool; Develocity pricing not public in what I found). devcontainers: vscode-dev-containers is archived (last push 2023-11-30), cli active (3.0k) [FACT]. INFERENCE: the gap is cross-tool correlation (shell history + env + lockfile + tool versions + git) on one machine; a senior can script it in a weekend, so craft is the only moat, consistent with the earlier null A/B.

## Evidence table

| Item | Stars / license / last push | Label | URL |
|---|---|---|---|
| atuin | 32,030 / MIT / 2026-10-19 | FACT | https://github.com/atuinsh/atuin |
| mise | 35,164 / MIT / 2026-10-19 | FACT | https://github.com/jdx/mise |
| direnv | 15,509 / MIT / 2026-10-12 | FACT | https://github.com/direnv/direnv |
| devbox | 12,417 / Apache-2.0 / 2026-10-15 | FACT | https://github.com/jetify-com/devbox |
| nix | 17,892 / LGPL-2.1 / 2026-10-19 | FACT | https://github.com/NixOS/nix |
| devcontainers/cli | 2,995 / MIT / 2026-10-19 | FACT | https://github.com/devcontainers/cli |
| vscode-dev-containers | 4,740 / MIT / archived, 2023-11-30 | FACT | https://github.com/microsoft/vscode-dev-containers |
| anthropics/sandbox-runtime | 5,592 / Apache-2.0 / 2026-10-17 | FACT | https://github.com/anthropics/sandbox-runtime |
| snyk/agent-scan (mcp-scan lineage; invariantlabs-ai/mcp-scan now 404; README links an Invariant "Introducing MCP-Scan" post, acquisition itself unverified) | 3,151 / Apache-2.0 / 2026-10-16; needs Snyk token | FACT (README) | https://github.com/snyk/agent-scan |
| docker/mcp-gateway | 1,607 / MIT / 2026-09-23 | FACT | https://github.com/docker/mcp-gateway |
| lasso-security/mcp-gateway | 396 / MIT / 2026-01-22 | FACT | https://github.com/lasso-security/mcp-gateway |
| invariantlabs-ai/invariant | 472 / Apache-2.0 / 2026-01-12 | FACT | https://github.com/invariantlabs-ai/invariant |
| NVIDIA/OpenShell | 16,676 / Apache-2.0 / 2026-10-19 | FACT | https://github.com/NVIDIA/OpenShell |
| microsoft/agent-governance-toolkit | 6,443 / MIT / 2026-10-19 | FACT | https://github.com/microsoft/agent-governance-toolkit |
| agentgateway | 5,410 / Apache-2.0 / 2026-10-19 | FACT | https://github.com/agentgateway/agentgateway |
| Palo Alto/Portkey, Boomi/Lunar.dev acquisitions | closed 2026-05-29 / n.d. | REPORTED (vendor blog + LinkedIn) | https://www.mintmcp.com/blog/agent-gateways-platform-engineering-teams |
| stryker-js / stryker-net / mutmut | 3,246 / 2,106 / 1,492; Apache/Apache/BSD-3; all active Oct 2026 | FACT | https://github.com/stryker-mutator/stryker-js |
| cloudbees-oss/smart-tests-cli | 27 / Apache-2.0 / 2026-10-19 | FACT | https://github.com/cloudbees-oss/smart-tests-cli |
| Launchable price ~US$12k/yr | n/a | REPORTED, third-party listing, unverified | https://www.getapp.com/it-management-software/a/launchable |
| SWE-Mutation benchmark | n/a | REPORTED (search summary) | https://aclanthology.org/people/jinbo-wang/ |
| MIST-RL, AdverTest | n/a | REPORTED | https://arxiv.org/html/2603.01409v1 ; https://www.arxiv.org/pdf/2602.08146 |
| GitHub Actions flaky builds study | 1,960 Java projects | REPORTED (arXiv) | https://arxiv.org/abs/2602.02307v1 |
| OSV: osv.dev / osv-schema | 2,978 / 272; Apache-2.0; active | FACT | https://github.com/google/osv.dev |
| Sigma | 11,275 / NOASSERTION / 2026-10-19 | FACT | https://github.com/SigmaHQ/sigma |
| semgrep-rules | 1,277 / NOASSERTION / 2026-10-16 | FACT | https://github.com/semgrep/semgrep-rules |
| Renovate | 22,797 / AGPL-3.0 / 2026-10-19 | FACT | https://github.com/renovatebot/renovate |
| Datadog flaky tests | vendor docs | REPORTED | https://docs.datadoghq.com/tests/flaky_tests/ |
| Netflix/dispatch (incident mgmt) | 6,487 / archived 2025-09 | FACT | https://github.com/Netflix/dispatch |
| HolmesGPT / k8sgpt / robusta / keep / kagent | 3.6k / 8.3k / 3.1k / 12.4k / 4.1k; mostly Apache/MIT; all active (keep NOASSERTION) | FACT | https://github.com/HolmesGPT/holmesgpt |
| mem0 / claude-mem / graphiti / cognee / letta | 67.4k / 101.7k / 32.0k / 32.5k / 25.2k; Apache-2.0; active | FACT | https://github.com/mem0ai/mem0 |
| Mem0 $24M raise | Oct 2025 | REPORTED | https://entrackr.com/snippets/mem0-raises-24-mn-in-series-a-round-10602602 |
| GitHub Copilot Memory | public preview, repo-scoped | FACT (docs) | https://docs.github.com/en/copilot/concepts/agents/copilot-memory |
| Copilot memory design | blog (vendor) | REPORTED | https://github.blog/ai-and-ml/github-copilot/building-an-agentic-memory-system-for-github-copilot/ |

Not found (404 via gh, may have different org/name; unverified): `mutmut/mutmut` (real repo is boxed/mutmut), `invariantlabs-ai/mcp-scan` (moved), Develocity OSS repo (commercial).

## Crowded / open / moat summary

| # | Candidate | Status | Credible clone resistance? |
|---|---|---|---|
| 1 | Local "what changed" diff | Piecemeal-crowded (free popular tools), integrated view open | Weak: craft only; weekend-clonable (INFERENCE) |
| 2 | Agent governance/audit | Most crowded; first-party and acquirers | Trust/distribution, held by incumbents; weak for newcomer |
| 3 | Verification oracle | Parts mature OSS; agent-specific packaging open | Moderate: craft plus benchmark credibility; data if it accumulates verdicts |
| 4 | Failure commons/format | Open (no healthy DB found) | Strongest in principle: network effect and standard-setting; but cold-start and no stable key |
| 5 | Verified-fix org memory | Generic memory crowded; verified-ops-fix niche open | Moderate: data per-org (lock-in), trust via verification; GitHub is the main threat |

## Ranking (best bet to worst) with cheapest falsifiable test

1. **(4) Failure commons/format** (high risk, high ceiling). Only candidate with a network-effect moat and no healthy incumbent; open-core fits (open schema + registry, paid hosted/private layer). Risk: no stable key, vendors hold private data, cold start. Test: take ~200 real public CI failure logs across ~20 repos (GitHub Actions, public), hand-write a signature schema, and measure whether signatures from 100 logs match the other 100 at >=30% with correct fix attribution. If it fails, there is no commons.
2. **(5) Verified-fix memory.** Differentiator is "verified" (fix re-run proven), which generic memory and Copilot Memory (code-location checks) do not claim. Test: for 30 recurring failures in 3 repos, give an agent with/without a retrieved verified-fix record; compare time/attempts-to-green on non-trivial cases (the null A/B result was on easy cases, so select hard ones).
3. **(3) Verification oracle.** Demand is plausible (weak LLM tests, SWE-Mutation) but free mutation tools exist. Test: run stryker/mutmut on 20 agent-authored PRs (public repos with Copilot/Codex/Claude PR trailers); if mutation score on agent-touched lines is not materially lower than on human-touched lines, the pain is unproven.
4. **(1) Local what-changed diff.** Weekend-clonable; null A/B. Test: a diff-only baseline (git + lockfiles + env hash) vs the product on 20 "worked yesterday" breakages; if baseline fixes >=80%, kill.
5. **(2) Agent governance.** Crowded, absorbed by Anthropic/Docker/Microsoft/NVIDIA/Snyk. Test: ask 10 target seniors which tool they use today and what is missing; if >=7 name a satisfactory first-party option, stop.

## Implications for OpsHero

- INFERENCE: avoid horizontal agent infrastructure (2) and generic memory (5-generic). Combine (4)+(5): open signature schema plus verified-fix records as the commons, with per-org private memory as the paid layer. The oracle (3) can be the *verification step* that makes records trustworthy (the "verified" attribute), not a standalone product.
- Cold-start is the existential risk: seed from public repos' CI logs (check license/ToS before ingesting).
- GitHub (Copilot Memory, gh-aw 5.4k stars) is the main absorber for (5); a cross-vendor, repo-agnostic, open format is the defense.

## Open questions

- Is there a stable, privacy-safe failure signature (normalized log fingerprint plus env keys) that generalizes across repos? Unproven.
- Did Snyk acquire Invariant Labs? README suggests lineage only; unverified.
- Develocity and Launchable actual prices: not found in primary sources.
- SWE-Mutation figures came via a search summary, not read in full.
- Sentry (45.6k stars, NOASSERTION license) and its AI debugging, plus Datadog/Develocity failure analytics, may already cover (4) privately; not examined in depth.
