# W2 - Evidence-based ranking of developer / DevOps pains (2025-2026)

Date: 2026-10-19. Labels: FACT (primary), REPORTED (secondary/vendor), INFERENCE (mine). Many primary reports were not directly fetchable (JetBrains 403, DORA PDF behind download, Sonar PDF unreadable): their numbers are REPORTED via press. Reddit was not reachable by search (only a third-party mirror); HN used via the Algolia API (HN skews anti-AI and senior: biased sample).

## Ranked pains (frequency / intensity / who pays)

1. **Verification and review of AI-written code (the bottleneck moved from writing to checking).** Highest frequency and intensity, and the only AI pain with clear paying buyers.
   - Frequency: 66% say AI is "almost right, but not quite", 45.2% say debugging AI code takes longer; experienced devs are the most distrustful (Stack Overflow 2025, ~31-34k responses per question, FACT).
   - Sonar 2026 (>1,100 devs, vendor survey, REPORTED): 96% do not fully trust AI code, only 48% always check it before commit, 38% say reviewing AI code takes more effort than reviewing a colleague's.
   - Faros (vendor, correlational, undefined metrics, REPORTED): high-AI teams show +199.6% time in PR review, +54% bugs/PR, +31.3% PRs merged with no review.
   - HN: "Ask HN: How do you review code nowadays?" (9 top-level comments, tiny) names intent-checking, undocumented project knowledge, review as shipping bottleneck. "After two years of vibecoding, I'm back to writing by hand" (865 pts): agent changes "look good in isolation" but degrade the whole; "I understand 5% of my codebase".
   - Pays today: CodeRabbit (seat $24/user/mo; Series C $143M at $1.5B, Aug 2026; ARR ~$40M is a Sacra estimate, REPORTED), GitHub Copilot review, Sonar. But the HN thread "There is an AI code review bubble" (351 pts) says it is commoditized: "Hard to beat free add-on". INFERENCE: money flows to generic PR-review bots, not to a new CLI diagnosis layer.

2. **CI/CD platform pain (GitHub Actions slow, flaky, fragile, expensive).** Very loud, and one of the few with money flowing.
   - HN (all >400 pts): "Pricing Changes for GitHub Actions" 802, "The Pain That Is GitHub Actions" 704, "I hate GitHub Actions with passion" 490, "GitHub Actions is slowly killing engineering teams" 404 (FACT that the threads exist; opinions are anecdotal).
   - Pays: faster-runner vendors. Blacksmith $45M Series B (press: 800 to 6,000 customers, REPORTED, conflicting), Depot $10M A. GitHub postponed its $0.002/min self-hosted fee (FACT, GitHub changelog) and cut hosted prices up to 39%.
   - Flaky tests: figures like Atlassian 150k dev-hours/yr are repeated vendor-blog numbers, unverified. Note the pain that is paid for is speed and cost of compute, not diagnosis.

3. **Agent safety, cost control, sandboxing (permissions, runaway spend, prompt injection).** Rising intensity.
   - HN: "AI agent bankrupted their operator" (1,467 pts): no hard spend cap on AWS, billing alerts do not help. Agent incidents recur in HN front-page items.
   - Pays: sandbox infra. Modal ~$300M ARR (REPORTED, multiple outlets, driven partly by sandboxes), Daytona $24M A, E2B $21M A (REPORTED). Buyers are agent builders and enterprises more than individual seniors.

4. **Loss of understanding / context loss / regressions with agents.** Intense among seniors, but mostly unpaid.
   - JetBrains 2025 (24,534 devs, REPORTED via InfoWorld): top AI concerns code quality 23%, limited understanding of complex code 18%, privacy 13%, skills erosion 11%, lack of context 10%.
   - HN: "Getting AI to work in complex codebases" 517 pts; "Claude Code is unusable for complex engineering tasks with Feb updates" 1,364 pts (regression after a model/tool change is a distinct pain: silent behavior drift).
   - Pays: indirectly via the agent vendors themselves (they absorb it as product work). INFERENCE: agent vendors will keep shipping memory/context natively, which squeezes third-party tools.

5. **PR/maintainer flooding by AI slop.** Loud, mostly open-source maintainers, unpaid. "PS3 emulator devs ask to stop AI PRs" (189 pts), Show HN slop detectors with 1-3 points. Dependabot opens 3-4M PRs/month yet merges ~1M (Octoverse, FACT): human approval, not generation, is the stall.

6. **On-call / incidents.** Real but a different market: PagerDuty ARR ~$501M, growth +1% (REPORTED via yespress, low quality); incident.io/Rootly "AI SRE" claims are vendor marketing. HN search for on-call returned nothing above 100 pts (weak signal here, possibly an artifact of my queries).

## Loud but unpaid vs paid
- Paid today: PR review bots (seat pricing), CI speed/cost (runners), agent sandboxes/infra, incident management platforms, SAST/quality gates (Sonar).
- Loud but unpaid: loss of codebase understanding, agent context loss, regressions after model updates, maintainer slop flooding, "vibe coding hangover", GitHub Actions YAML misery (people complain, but switching spend goes to runner vendors).

## Contrary evidence (overstated pains)
- METR RCT: 16 devs, 246 tasks, 19% slower while believing 20% faster (REPORTED, tiny sample, early-2025 tools). The Feb 2026 follow-up admits selection bias made data unreliable (REPORTED). Do not cite either as proof of a verification crisis.
- DORA 2025 (~5,000 respondents, REPORTED via InfoQ): AI now correlates positively with throughput; instability also rises; AI is an "amplifier" of existing org strengths. Over 80% believe AI raised productivity. Secondary source quoted a conflicting "-7.2% stability" figure, unverified.
- Faros/LinearB/CodeRabbit "AI code has more bugs" numbers all come from vendors that sell the fix. The Faros post itself says correlational.
- AIDev studies (456k agent PRs, arXiv 2507.15003): agent PR acceptance varies 45-86% by agent and task, human PRs 82.6% in one study (REPORTED, different subsets): agents are not uniformly rejected.
- HN is self-selected and anti-AI ("Why is the HN crowd so anti-AI?" 463 pts); Octoverse notes agent PRs skew to larger repos and does not measure review load (FACT, own caveat).
- The user's own A/B null on easy cases fits: pains appear at scale/complexity, not on easy cases.

## Evidence table

| Claim | Label | URL |
|---|---|---|
| 66% almost-right, 45.2% debugging longer, trust 33/46% | FACT | https://survey.stackoverflow.co/2025/ai |
| Sonar 2026: 96% distrust, 48% always verify, 38% harder review | REPORTED (vendor) | https://www.sonarsource.com/company/press-releases/sonar-data-reveals-critical-verification-gap-in-ai-coding/ |
| DORA 2025 amplifier, ~5k respondents | REPORTED | https://www.infoq.com/news/2025/09/dora-state-of-ai-in-dev-2025 |
| JetBrains 24,534 devs, concern breakdown | REPORTED | https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/ |
| Octoverse: Dependabot 3-4M PRs vs ~1M merged; 1M+ Copilot agent PRs | FACT | https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/ |
| Faros review-time/bugs stats | REPORTED (vendor) | https://www.faros.ai/blog/ai-code-quality-senior-engineer-review-burden |
| METR 19% slower, n=16 | REPORTED | https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/ |
| Agentic PR acceptance studies | REPORTED | https://arxiv.org/html/2602.08915v1 and https://www.arxiv.org/pdf/2602.04226 |
| HN: Actions pricing/pain threads | FACT (threads) | https://news.ycombinator.com/item?id=46291156 , https://news.ycombinator.com/item?id=43419701 , https://news.ycombinator.com/item?id=46614558 , https://news.ycombinator.com/item?id=46908491 |
| GitHub postpones self-hosted fee; hosted -39% | FACT | https://github.blog/changelog/2025-12-16-coming-soon-simpler-pricing-and-a-better-experience-for-github-actions/ |
| HN: AI code review bubble | FACT (thread) | https://news.ycombinator.com/item?id=46766961 |
| HN: how do you review code nowadays | FACT (thread) | https://news.ycombinator.com/item?id=50076575 |
| HN: two years of vibecoding | FACT (thread) | https://news.ycombinator.com/item?id=46765460 |
| HN: agent bankrupted operator | FACT (thread) | https://news.ycombinator.com/item?id=48500012 |
| HN: Claude Code unusable after Feb updates | FACT (thread) | https://news.ycombinator.com/item?id=47660925 |
| CodeRabbit Series C, $24 seat, ARR estimate | REPORTED | https://sacra.com/c/coderabbit/ |
| Blacksmith/Depot funding | REPORTED | https://www.tamradar.com/funding-rounds/blacksmith-series-b-45m |
| Modal/Daytona/E2B sandbox money | REPORTED | https://www.e2b.dev/blog/series-a |
| Atlassian DevEx 2025, n=3,500, no figures retrieved | FACT (partial) | https://www.atlassian.com/teams/software-development/state-of-developer-experience-2025 |

## Implications for OpsHero (INFERENCE)
- The best-evidenced, most durable pain is the verification gap, but the paid slot is crowded (CodeRabbit, Copilot, Sonar, Cursor). A generic reviewer loses; a deterministic, agent-callable evidence layer (checks, provenance, policy) that sits under many agents might not.
- CI diagnosis is weak: people pay for CI speed/cost, not explanation, and platforms are absorbing diagnosis. This matches the earlier A/B null.
- Agent guardrails (spend, permissions, blast radius) and "did the agent actually do what it claimed" have rising intensity and real buyers, but sandbox infra is a capital-heavy market.
- Open-core fit: individual seniors will adopt free CLI; paying buyers are teams/orgs (governance, audit, shared policy).

## Open questions
- Who is the buyer for a seniors' CLI: individual (no budget evidence) or platform/DevEx team (DORA: 90% have internal platforms, 76% dedicated platform teams, REPORTED)?
- Does verification pain persist as models improve (Faros eval: Opus 4.6 +0.08 vs Sonnet 3.7 -0.70, method undisclosed)?
- Real r/ExperiencedDevs and r/devops volume not measured (tooling gap).

## 3 product ideas with cheap falsifiable tests
1. **Agent-claim verifier**: CLI that replays an agent's claimed result ("tests pass, bug fixed") from the diff and logs, emits a signed pass/fail evidence block into the PR. Test: run on 50 recent agent PRs in 5 public repos; falsified if it flags fewer than ~10% real discrepancies that maintainers confirm, or if reviewers say it changes no decision (5 maintainer interviews).
2. **Agent blast-radius / spend guard**: wrapper enforcing per-task cloud spend, credential scope and network policy for any coding agent. Test: landing page + OSS prototype; falsified if fewer than 20 stars-with-usage or 5 teams willing to pilot within 4 weeks, or if Claude Code/Codex native permissions already meet the need in interviews.
3. **Review triage by risk**: ranks the PR queue by blast radius (touched ownership, test coverage, undocumented-knowledge hits) so seniors review what matters. Test: replay 200 merged PRs, check if the top-decile risk score predicts later reverts/hotfixes better than size alone; falsified if AUC gain over PR-size baseline is below 0.05.
