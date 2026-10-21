# W3 - AI SRE / DevOps / Platform tooling landscape (as of 2026-10-19)

Labels: FACT = primary source (gh API queried today, pricing page); REPORTED = secondary/vendor; INFERENCE = mine. Search was thin: no HN/Reddit practitioner threads were returned; practitioner evidence is therefore weak (see Open questions).

## Ranked key findings

1. **Incident RCA is a crowded, well-funded, closed-source market with hyperscaler price pressure. Do not compete head-on.** Resolve AI: $125M Series A at $1B (Dec 2025, TechCrunch, REPORTED); a single blog source claims $40M extension at $1.5B and total >$190M (REPORTED, unverified). AWS DevOps Agent is priced at $0.0083/agent-second (~$30/h active; AWS example: 10 x 8 min investigations = ~$39.84/month) (pricing page, FACT; GA date 2026-03-31 from a third-party blog, REPORTED). Datadog Bits AI SRE moved to shared credits ($500 per 500 credits; ~6.5 credits/investigation per one secondary source, so about $6.50, REPORTED). INFERENCE: price per investigation is falling toward single-digit dollars; a paid generic RCA product has no pricing room.
2. **The open-source incumbents in K8s diagnosis are alive, but with very different health.** HolmesGPT (CNCF Sandbox since 2025-10-08 per CNCF page; Apache-2.0): 3,636 stars, 538 forks, pushed today, release 0.45.0 on 2026-10-15, 114 commits in the last 3 months across ~5+ authors, mostly Robusta staff (FACT, gh). K8sGPT (Apache-2.0): 8,256 stars, release v0.4.40 on 2026-10-09, but 92 commits in 3 months of which 21 are renovate[bot], long tail of volunteers (FACT, gh). INFERENCE: K8sGPT is popular but low-depth (a pattern "analyzer", no multi-source agent); HolmesGPT is the real agentic OSS reference, backed by a vendor (Robusta, MIT, 3,116 stars). The OSS gap is NOT "an AI RCA agent"; it exists.
3. **Practitioner skepticism is concentrated on novel failures, trust, and coordination, not on summarization.** Rootly (vendor) itself says autonomous diagnosis of unfamiliar failures, confident wrong answers and uninstrumented systems "don't work yet"; it publishes no accuracy figures (WebFetch of its blog, REPORTED). Lorin Hochstein (practitioner blog) argues agents fixate on wrong hypotheses and that nobody builds incident coordination/common ground (REPORTED, one author). No independent accuracy benchmark with sample size was found (unverified).
4. **CI-failure fixing is being absorbed by CI vendors, as the team suspected.** Nx self-healing is billed in credits (pricing page shows $19/active contributor/month, $5.50/10k credits; vendor claims "50% less time to green", marketing, REPORTED). CircleCI Chunk: beta, uses CircleCI credits plus own model tokens, post-beta price not found (REPORTED). GitHub gh-aw (Agentic Workflows): MIT, 5,420 stars, pushed today (FACT, gh). INFERENCE: a standalone CI-fix tool competes with free GitHub-native, markdown-defined workflows.
5. **Agent-facing tool plumbing (MCP servers, agent-runtime on K8s) is where OSS momentum is, and it is commodity.** grafana/mcp-grafana 3,575 stars Apache-2.0; kagent (CNCF-style K8s agent framework) 4,052 stars; PagerDuty MCP 79 stars; Rootly MCP 45 stars (FACT, gh). INFERENCE: vendors ship their own MCP servers; wrappers add little.
6. **IaC: the best-known OSS drift tool is in decline; AI-for-IaC is mostly vendor-embedded.** driftctl (snyk, 2,666 stars) was described as maintenance mode since 2023, last push 2026-09-21 (a LinkedIn post, REPORTED; push date FACT). New drift tools (drifty, tfdrift, terraform-review-agent) are tiny/unverified single-source mentions (REPORTED). Spacelift Intent: 138 stars (FACT). OpenTofu 30,542 stars MPL-2.0, Pulumi 25,794 Apache-2.0, checkov 9,078, trivy 38,508 are mature and active (FACT). INFERENCE: scanners (checkov/trivy/OPA 12,386) own "facts"; an LLM layer over their output is a thin wrapper.
7. **Platform engineering: Backstage dominates OSS (34,653 stars, Apache-2.0, FACT) but is costly to run.** Claims of 3-6 engineers for 12 months and ~89% IDP share are from vendor/blog sources, unverified (REPORTED). Crossplane 12,180 stars (FACT). Netflix/dispatch (incident management OSS, 6,487 stars) is archived, last push 2025-09 (FACT); Grafana OnCall OSS archived (3,882 stars, AGPL, FACT). INFERENCE: OSS incident management/on-call has a real hole, left by archived projects.

## Evidence table

| Item | Claim | Label | URL |
|---|---|---|---|
| K8sGPT | 8,256 stars, Apache-2.0, push 2026-10-17, v0.4.40 | FACT | https://github.com/k8sgpt-ai/k8sgpt |
| HolmesGPT | 3,636 stars, Apache-2.0, 0.45.0 (2026-10-15) | FACT | https://github.com/HolmesGPT/holmesgpt |
| HolmesGPT CNCF Sandbox | accepted 2025-10-08 | FACT/REPORTED (search snippet) | https://www.cncf.io/projects/holmesgpt/ |
| Robusta | 3,116 stars MIT | FACT | https://github.com/robusta-dev/robusta |
| Keep (AIOps) | 12,419 stars, license NOASSERTION, last push 2026-09-28 | FACT | https://github.com/keephq/keep |
| kagent | 4,052 stars Apache-2.0 | FACT | https://github.com/kagent-dev/kagent |
| gh-aw | 5,420 stars MIT | FACT | https://github.com/github/gh-aw |
| mcp-grafana | 3,575 stars | FACT | https://github.com/grafana/mcp-grafana |
| Backstage | 34,653 stars | FACT | https://github.com/backstage/backstage |
| Crossplane | 12,180 stars | FACT | https://github.com/crossplane/crossplane |
| OpenTofu / Pulumi | 30,542 / 25,794 stars | FACT | https://github.com/opentofu/opentofu , https://github.com/pulumi/pulumi |
| Netflix dispatch | archived | FACT | https://github.com/Netflix/dispatch |
| Resolve AI funding | $125M A at $1B | REPORTED | https://techcrunch.com/2025/12/19/ex-splunk-execs-startup-resolve-ai-hits-1-billion-valuation-with-series-a |
| Resolve AI $1.5B, customers (Coinbase 72% faster, vendor-sourced) | unverified | REPORTED | https://www.beri.net/article/resolve-ai-190m-ai-sre-production-incidents-2026 |
| AWS DevOps Agent price | $0.0083/agent-second | FACT | https://aws.amazon.com/devops-agent/pricing |
| Datadog credits | $500/500 credits | REPORTED (comparison sites) | https://datadoghq.com/pricing |
| Nx pricing/self-healing | credit-based, vendor claims | REPORTED | https://nx.dev/pricing |
| CircleCI Chunk | beta, price unknown | REPORTED | https://circleci.com/product/chunk |
| Rootly "do AI SRE agents work" | vendor, no data | REPORTED (vendor) | https://rootly.com/blog/do-ai-sre-agents-actually-work |
| Hochstein critique | agents fixate; no coordination | REPORTED (1 author) | https://surfingcomplexity.blog/2026/02/14/lots-of-ai-sre-no-ai-incident-management/ |

Not covered/unverified: Cleric, Komodor, incident.io, PagerDuty AI, Port/Humanitec traction and pricing, Harness (no usable source found); Terraform/Spacelift AI claims.

## Implications for OpsHero

- Avoid: generic "AI explains your incident/CI failure" (priced to ~zero by AWS/Datadog/Nx/GitHub; weekend-cloneable).
- Open-core OSS is credible only where vendors are structurally unwilling to serve: cross-vendor, local-first, evidence/trust layers, and incident coordination (archived Dispatch/OnCall suggest unmet OSS demand, though maintenance cost may be why they died).
- Experienced SREs' stated trust pattern (REPORTED, weak): they accept agents that retrieve, correlate and summarize with a human reviewing; they distrust confident autonomous RCA. A product that makes agent conclusions verifiable beats one that makes them more fluent.
- INFERENCE: evals are the missing artifact; vendors publish none. A replayable incident corpus + scorer for agents is OSS-natural and not cloneable by a weekend script because the value is the data.

## Open questions

- Do senior SREs actually pay for/keep using any AI SRE after trial? No retention or independent accuracy data found.
- HN/Reddit threads were not retrievable; run manual searches (r/sre, r/devops, HN "Ask HN: AI SRE") before committing.
- Why did Dispatch and OnCall OSS get archived (cost, lack of demand, acquisition)?
- Real HolmesGPT adoption (stars are not usage); its own eval results not found.

## 3 product ideas with cheap falsifiable tests

1. **Agent incident-eval harness (OSS) + hosted corpus**: replayable real/synthetic outage bundles (logs, metrics, manifests, deploys) with ground-truth cause; scores HolmesGPT/K8sGPT/Claude Code on yours. Test: build 15 incidents from public postmortems; run 3 agents; publish. Falsified if the scores do not differ meaningfully between agents or if fewer than ~5 SRE teams ask to run it on their own incidents within 4 weeks of posting.
2. **Evidence-linked conclusions ("claim -> query -> raw output" verifier)** wrapping any agent's RCA, flagging claims with no supporting tool output. Test: take 30 agent RCAs from idea 1; measure whether the verifier catches wrong ones at >=70% recall with <20% false flags; also show 5 SREs and see if verified output changes their accept/reject rate. Falsified if recall is low or SREs do not change decisions.
3. **Incident coordination CLI/state file for humans + agents** (active hypotheses, owner, ruled-out list, stale-summary alerts; Hochstein gap). Test: run it in 3 game-day exercises with a small team versus a Slack baseline; falsified if time-to-shared-understanding or duplicated investigations do not improve, or if no team reuses it in a real incident within 6 weeks.
