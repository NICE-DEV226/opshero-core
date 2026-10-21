# W5 - Open-source / open-core dev tools: what won, what died (2026-10-19)

Labels: FACT = primary source, REPORTED = secondary/vendor/estimator, INFERENCE = mine. Not covered (unverified): Zed revenue, Warp pricing, Nx Cloud revenue, Depot revenue, Atuin revenue, mise revenue beyond sponsor page, Turborepo revenue. Star counts from `gh api` on 2026-10-19; stars are not revenue and can be inflated (not checked).

## Ranked key findings

1. **Distribution + a closed hosted/enterprise layer won; the CLI itself rarely monetizes.** Winners (Sentry, PostHog, Supabase, Grafana, Tailscale) sell hosting, scale, or control-plane around a free core. PostHog: free self-serve, no outbound sales, ~2% of users pay (REPORTED, Sacra/Stripe). Grafana >$400M ARR with 7,000+ customers (FACT, company press release, vendor claim). Supabase ~$170M ARR est., $10.5B valuation, >60% of new DBs created by AI tools (REPORTED; Sacra estimate, Orrick for round).
2. **Bottom-up adoption that does not convert is the commonest death.** Earthly (12k stars, MPL): shut down commercial services and froze OSS on 2025-07-16. Its own post cites big orgs reluctant to deploy company-wide, users getting the speed without paying Satellites, CI compute seen as commodity (FACT, earthly.dev post). Earlier it had already killed Earthly CI (2023). Last repo push 2025-10-23 (FACT, gh).
3. **Tool-chain tools get acquired by AI labs, not monetized.** Bun: "makes $0 in revenue", ~$26M raised, joined Anthropic Dec 2025 (FACT, Bun blog). Astral (uv/ruff/ty): joining OpenAI Codex, announced 2026-03-19, closing pending as far as I could find (FACT announcement; closing unverified). Astral funded by Accel/a16z (FACT). INFERENCE: the exit for an agent-adjacent CLI is strategic acquisition by an agent vendor, which is a bet on a buyer, not a business.
4. **AI is already eroding revenue where value is "docs/knowledge/code generation".** Tailwind Labs: usage at record high, revenue down ~80%, docs traffic down ~40% since 2023, 75% of engineers laid off (REPORTED, devclass quoting CEO, Jan 2026). Retool survey (n=817, vendor, 36% devs): 35% replaced at least one SaaS tool with custom build; only 8% use AI code unchanged (REPORTED, bias: vendor sells build platform).
5. **Licensing is a weapon and a liability; forks follow quickly.** HashiCorp BSL (Aug 2023) -> OpenTofu (MPL, 30.5k stars, active) -> IBM bought HashiCorp ~$6.4B, closed 2025-02-27 (REPORTED). Semgrep restricted rules Dec 2024 -> Opengrep fork by 10+ security vendors, 3.2k stars vs 17.1k (REPORTED InfoQ/TheNewStack, FACT stars). Sentry went BSL then FSL (2-year conversion to MIT/Apache) and stayed independent (FACT, sentry blog). Warp went AGPL client + closed Oz agent platform Apr 2026 (REPORTED). RTK uses Apache CLI + BUSL engine (FACT, README).
6. **Sponsorship-funded solo/duo tools survive but stay tiny.** mise (35k stars): sponsor tiers $500/$1.5k/$3k per month, capped slots (FACT, jdx.dev/sponsors). Renovate: free community edition, Mend sells Enterprise Edition (horizontal scale, support) (REPORTED, Mend/SD Times). Atuin: VC-backed (amount unverified), closed-source Hub for sync/sharing, Desktop to be OSS (REPORTED forum FAQ).
7. **Agent-token tools get huge star counts fast, with no visible revenue.** rtk-ai/rtk: repo created 2026-01-22, 83.9k stars, 5.3k forks, 1.5k open issues in 9 months; no paid tier on README; telemetry opt-in; savings claim "60-90%" is bash output only, token numbers estimated bytes/4 (FACT, README + gh). INFERENCE: pure adoption proof, zero monetization proof; also shows a CLI wedge can be cloned (a Rust proxy is a weekend of agent work).
8. **OSS maintenance load is rising with AI.** curl ended bug bounty Jan 2026; tldraw auto-closes external PRs; Jazzband sunset Mar 2026 (REPORTED, secondary blogs; verify primaries). INFERENCE: open-core with community-contribution strategy costs more now; "open for use, closed for contribution" is the new default.

## Evidence table

| Tool | License (gh) | Stars | Adoption/distribution | Monetization | Outcome | Source |
|---|---|---|---|---|---|---|
| Sentry | FSL (NOASSERTION) | 45.6k | SDKs bottom-up | SaaS usage tiers | ~$128M ARR est. 2023; $3B val 2022 (REPORTED) | https://sacra.com/research/sentry-disrupting-new-relic ; https://blog.sentry.io/introducing-the-functional-source-license-freedom-without-free-riding |
| PostHog | MIT core + EE (NOASSERTION) | 40.3k | Free self-host, PLG, no sales | Usage-based | ~$58M ARR est. Feb 2026, 3x growth claim (REPORTED) | https://sacra.com/research/58m-year-hedgehog-of-product-analytics ; https://www.nasdaq.com/articles/meet-posthog-startup-just-raised-70m-series-d-led-stripe-single-tweet |
| Supabase | Apache-2.0 | 111.8k | Postgres + AI app builders default | Hosted | $10.5B val Jun 2026 (REPORTED) | https://www.orrick.com/en/News/2026/06/Supabase-Raises-500-Million-at-10-5-Billion-Valuation-to-Accelerate-Lead-in-Agentic-Infrastructure ; https://sacra.com/research/supabase-170m-year-growing-221-yoy/ |
| Grafana | AGPL-3.0 | 77.4k | Dashboards, OTel/Prometheus ecosystem | Cloud + enterprise | >$400M ARR (FACT-vendor) | https://grafana.com/press/2025/09/30/grafana-labs-surpasses-400m-arr-and-7000-customers-gains-new-investors-to-accelerate-global-expansion/ |
| HashiCorp/OpenTofu | BSL / MPL-2.0 | 50.0k / 30.5k | Fork under LF/CNCF | n/a | IBM acquisition ~$6.4B (REPORTED) | https://linuxiac.com/ibm-completes-6-4-billion-acquisition-of-hashicorp/ |
| Semgrep/Opengrep | LGPL-2.1 both | 17.1k / 3.2k | Vendor consortium fork | Semgrep: commercial engine+rules | Split ecosystem | https://www.infoq.com/news/2025/02/semgrep-forked-opengrep |
| Astral | Apache/MIT | uv 90.9k, ruff 50.2k | Speed, drop-in replacement | None visible; pyx registry beta | Joining OpenAI, pending close | https://astral.sh/blog/openai |
| Bun | MIT (NOASSERTION shown) | 96.3k | Speed, runtime | $0 revenue | Acquired by Anthropic | https://bun.com/blog/bun-joins-anthropic |
| Warp | AGPL (UI MIT) | 65.5k | Terminal + AI | Subscriptions, closed Oz | OSS client Apr 2026 (REPORTED) | https://www.helpnetsecurity.com/2026/04/30/warp-open-source-client/ |
| Zed | NOASSERTION | 91.9k | Editor | AI features; revenue unverified | $32M Series B Aug 2025 (REPORTED) | https://startupintros.com/news/2025-08-01-zed-industries-series-b |
| Tailscale | BSD-3 (client) | 37.8k | Dev love -> teams | Per-seat | $160M Series C Apr 2025, >100% rev growth (REPORTED) | https://thelogic.co/briefing/tailscale-raises-us160m-series-c-for-workplace-vpn/ |
| Dagger | Apache-2.0 | 16.4k | Docker founders | Cloud $50/mo/10 users flat (2024) | $30M raised total, no round since 2022 found (REPORTED) | https://dagger.io/blog/new-dagger-cloud-pricing |
| Earthly | MPL-2.0 | 12.1k | Bottom-up | Satellites | Shut down Jul 2025 (FACT) | https://earthly.dev/blog/shutting-down-earthfiles-cloud/ |
| Nx | MIT | 29.4k | Monorepo standard | Nx Cloud, self-healing CI | 36M npm dl/mo, >50% AI fixes "useful" (vendor claims) | https://nx.dev/blog/nx-2026-roadmap |
| Turborepo | MIT | 31.2k | Acquired by Vercel 2021 | Remote cache via Vercel | Absorbed | https://turborepo.dev/blog/joining-vercel |
| Depot | closed | n/a | CI/build speed, PostHog as customer | Usage SaaS | $10M Series A Mar 2026 (REPORTED, aggregators; revenue unverified) | https://www.trysignalbase.com/news/funding/depot-raises-10m-series-a |
| Atuin | MIT | 32.0k | Shell history | Hub sync (closed) | VC-backed, amount unverified | https://forum.atuin.sh/t/frequently-asked-questions/1031 |
| mise / direnv | MIT / MIT | 35.2k / 15.5k | Dev-env standard | Sponsors | Healthy, small | https://jdx.dev/sponsors |
| Renovate | AGPL-3.0 | 22.8k | Free hosted app | Mend Enterprise | Bundled in AppSec vendor | https://sdtimes.com/open-source/mend-io-launches-enterprise-version-of-dependency-management-tool-renovate/ |
| RTK | Apache CLI + BUSL engine | 83.9k | brew/winget/curl, 18 agents | None shown | Stars, no revenue evidence | https://github.com/rtk-ai/rtk |
| Tailwind (AI case) | MIT | n/a | n/a | Paid UI kits via docs | Revenue -80% (REPORTED) | https://www.devclass.com/2026/01/08/tailwind-labs-lays-off-75-percent-of-its-engineers-thanks-to-brutal-impact-of-ai/ |

## What stays defensible when code is cheap (INFERENCE, supported above)
Hosted state and data (Sentry, PostHog, Supabase, Grafana), network effects/identity (Tailscale), trust and compliance for orgs, integrations living inside the agent/vendor surface, and distribution defaults (Supabase as AI builders' default DB). Not defensible: thin local CLIs and wrappers (RTK-class, Earthly-class), docs-funnel businesses (Tailwind).

## Five replicable playbooks for 3-8 seniors (INFERENCE)
1. **Free core + closed hosted state/sync layer** (Atuin, Sentry, PostHog). Preconditions: the tool produces data worth keeping across machines/teams; self-host is painful. Milestone: 1,000 weekly active CLI installs and 20 teams using sync within 90 days.
2. **Default-in-agents distribution** (Supabase, RTK, Bun). Preconditions: works as hook/MCP/skill for Claude Code, Codex, Cursor with no config. Milestone: listed in 3 agent ecosystems; 500 installs/week without paid acquisition.
3. **Team-tier sold bottom-up, flat per-team price** (Dagger $50/10 users, Tailscale). Preconditions: a single-dev workflow with obvious multi-dev value (shared policy, shared cache). Milestone: 10 paying teams at a self-serve price with no sales call.
4. **Sponsor/enterprise-support micro-business** (mise, Renovate/Mend). Preconditions: tool is infrastructure companies already depend on. Milestone: 3 company sponsors at $500+/mo.
5. **Strategic-exit wedge** (Bun, Astral, Turborepo): own an indispensable primitive an agent vendor needs. Preconditions: category leader on speed/correctness metric; accept buyer dependency. Milestone: top-1 benchmark and one agent vendor integrating you.

## Implications for OpsHero
- A diagnosis CLI is a thin local tool: closest analogs are Earthly (commoditized CI layer) and RTK (stars, no revenue). Avoid selling the CLI; sell persistent cross-run/cross-team state (flaky history, failure fingerprints) that a clone cannot rebuild in a weekend.
- Nx already ships "self-healing CI" and claims >50% useful fixes (vendor claim), so incumbent absorption is confirmed.
- Pick license deliberately: FSL (Sentry) beats BSL/AGPL churn for a small team; keep the cloud closed.
- Budget for maintainer load from AI PRs; consider closed contributions.

## Open questions
- Did the OpenAI/Astral deal close? Revenue of Astral pyx?
- Real conversion of agent-tool stars (RTK) to paid; is star growth organic?
- Zed, Warp, Depot, Nx Cloud actual revenue (unverified).
- Retool sample bias; no independent 2026 data on OSS-clone prevalence found.

## Three ideas to test (cheap, falsifiable)
1. **Cross-run failure memory as hosted state.** Test: ship a free CLI that uploads anonymized failure fingerprints to a shared store for 20 repos; success = at least 30% of repos query history on 3+ distinct days and 5 say they would pay $50/team/mo; fail otherwise.
2. **Agent-native hook that wins by measured outcomes.** Test: on 50 real failing CI runs (not easy cases), compare agent fix rate with vs without the tool; ship only if the delta is >=15 points with a CI that excludes zero.
3. **Sponsor/team-tier smoke test.** Test: landing page with $49/team/mo and 3 sponsor tiers, 200 targeted dev visits from a launch (Show HN, agent communities); success = >=5 paid pre-orders or LOIs; fail = zero.
