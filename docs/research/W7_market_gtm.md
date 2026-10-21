# W7 — Go-to-market and geography (francophone West Africa founders, OSS devtool)

Date: 2026-10-19. Search is US-only and thin; several items are unverified (marked). Labels: FACT = primary, REPORTED = secondary/vendor, INFERENCE = mine.

## Ranked key findings

1. **Payout/payment rails are the sharpest, most concrete constraint, and Burkina Faso is the weak point.** Stripe lists Côte d'Ivoire only as "Extended network" (via Paystack), and does not list Burkina Faso or Senegal. GitHub Sponsors lists Côte d'Ivoire and Senegal but NOT Burkina Faso. Lemon Squeezy bank payouts list Côte d'Ivoire/Senegal/Benin/Niger but not Burkina Faso (PayPal payout is "200+ countries", list not shown). The GitHub Secure Open Source Fund requires being in a Sponsors-supported region. INFERENCE: a Burkina-resident legal entity/bank is a real blocker for global billing and some grants; a Côte d'Ivoire (or other) entity, or a Stripe Atlas/Delaware-style entity, is likely a prerequisite. Unverified: Atlas/Mercury eligibility for Burkinabè founders, Polar.sh payout list (a comparison blog says parts of Africa unsupported).
2. **The developer population is growing fast but is overwhelmingly anglophone/East-West-coastal, and francophone West Africa is small.** GitHub says Africa+Middle East added ~3.4M developers 2024-25, standouts Nigeria, Egypt (Turkey too); Nigeria/Egypt/Kenya/Morocco projected to add millions. No francophone West Africa counts found (unverified). Growth is mostly learners and juniors (bootcamps, mobile, LLMs). INFERENCE: the experienced-engineer, platform/devops buyer segment locally is thin and mostly employed by banks, telcos, and outsourcers with local-budget constraints.
3. **"Offline-first / cheap to run" is a plausible wedge in Africa, not shown as a global wedge.** Evidence for local cost pain is real but old/vendor-skewed (2019 A4AI: 1GB ~7-8% of avg monthly income in Africa; Burkina 7.8% after cuts; Nigerian FX/dollar-card trouble, AWS naira billing from Jan 2025 — vendor sources). I found NO evidence that global senior buyers pay for low-bandwidth/offline. INFERENCE: for global markets it is a feature (air-gapped/regulated/self-host/local-model is a separate, real segment, but that is "sovereignty", not "Africa"). It is also trivial for a competitor to copy: a wedge, not a moat.
4. **Funding is real but narrow, and mostly not suited to a commercial open-core tool.** NLnet NGI Zero Commons: EUR 5k-50k, anyone may apply per secondary summaries, "European dimension" language; the 13th/final call closed 2026-06-01 (FACT, nlnet.nl). Sovereign Tech Agency: scope is base infrastructure (libraries, package managers, protocols), explicitly not user-facing apps; anyone worldwide may apply (REPORTED); min figure conflicting (EUR 150k official page vs 50k later report). GitHub Secure OSF: USD 10k per project, tranches 6/2/2k, plus Azure credits; needs Sponsors-supported region, traction, governance; 2024 call info only, no current round found. INFERENCE: grants fit only if OpsHero ships a genuinely commons-type component (a parser/spec/library), not the commercial product. Sponsors scale: >$100M total, 70k+ maintainers (REPORTED by GitHub via HelpNetSecurity); ~40% from orgs; no median found, so assume most earn little. Tidelift 2024 (vendor, n~400-440): ~60% maintainers unpaid.
5. **The one strong African-founded OSS devtool precedent is Better Auth (Ethiopia), and what worked is not geography.** 30.4k stars, MIT, active today (gh api), $5M seed (Peak XV, YC) with open core + paid cloud/enterprise (REPORTED). Its success came from a sharply scoped, TypeScript-native, high-demand niche, rapid shipping, and agent-era ecosystem fit; being African was PR color. Ushahidi (Kenya, 730 stars, NOASSERTION license) is an older civic-tech example, not a devtool. I found no francophone African global devtool (unverified/absent).
6. **Distribution channels are cheap and location-independent.** Homebrew-core excludes unstable-release software; notability thresholds not retrieved (unverified). cargo/npm/pip/winget/Nix/MCP registries are open. INFERENCE: distribution is not the bottleneck; attention and trust are. Community evidence for OSS devtools specifically was not found in this pass (unverified).

## Evidence table

| # | Claim | Label | URL |
|---|---|---|---|
| a | Africa+ME +3.4M developers 2024-25; Nigeria/Egypt standouts; >180M GitHub devs | FACT (GitHub) | https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/ |
| b | Nigeria ~445k accounts, Kenya ~167k (Innovation Graph via OpenUK; Nigeria figure typo-ridden) | REPORTED | https://openuk.uk/wp-content/uploads/2026/05/Open-Source-Africa-Report.pdf |
| c | Stripe: Côte d'Ivoire "Extended network" via Paystack; Burkina Faso, Senegal not listed | FACT | https://stripe.com/global |
| d | GitHub Sponsors: Côte d'Ivoire, Senegal listed; Burkina Faso, Benin, Mali, Togo not | FACT | https://docs.github.com/en/sponsors/getting-started-with-github-sponsors/about-github-sponsors |
| e | Lemon Squeezy bank payouts: CI, Senegal, Benin, Niger yes; Burkina no | FACT | https://docs.lemonsqueezy.com/help/getting-started/supported-countries |
| f | GitHub Secure OSF: $10k, region + traction + governance rules | FACT (GitHub) / 2024 call | https://github.com/open-source/github-secure-open-source-fund |
| g | NLnet NGI0 Commons: EUR 5-50k; final call closed 2026-06-01 | FACT | https://nlnet.nl/commonsfund/ |
| h | Sovereign Tech: base infrastructure only, global applicants, min EUR 50k vs 150k conflict | REPORTED/FACT mixed | https://www.sovereign.tech/programs/fund ; https://www.sovereigntechfund.de/programs/applications |
| i | Sponsors >$100M, 70k+ maintainers, ~40% org-funded | REPORTED (GitHub numbers) | https://www.helpnetsecurity.com/2026/07/21/open-source-github-sponsors-100-million/ |
| j | ~60% maintainers unpaid (vendor survey, n~400) | REPORTED, vendor | https://thenewstack.io/open-source-paid-maintainers-keep-code-safer-survey-says/ |
| k | Better Auth: $5M seed, open-core | REPORTED | https://launchbaseafrica.com/2025/06/26/rare-african-solo-founder-behind-better-auth-raises-5m-to-rethink-developer-authentication/ |
| l | Better Auth 30,369 stars, MIT, pushed 2026-10-19 | FACT (gh api) | https://github.com/better-auth/better-auth |
| m | Africa 1GB ≈ 7-8% avg income (2019); Burkina 7.8% | REPORTED, dated | https://webfoundation.org/our-work/projects/alliance-for-affordable-internet/ |
| n | Nigeria dollar-card/FX friction; AWS naira billing 2025 | REPORTED, vendor-heavy | https://restofworld.org/2023/foreign-currency-shortages-are-cutting-nigerians-off-from-apple-music-aliexpress-and-more/ ; https://thecondia.com/aws-naira-payments-nigeria-cloud-market/ |
| o | Homebrew-core needs stable release (notability rules not retrieved) | FACT partial | https://docs.brew.sh/Acceptable-Formulae |
| p | Stack Overflow 2025: 49k+ responses, 177 countries, SO-channel recruited; Africa absent in results | REPORTED | https://survey.stackoverflow.co/ |

## Implications for OpsHero

- Treat geography as an operating-cost advantage (low burn, long runway), not a product wedge or a market. Local buyers are too few and too low-budget to carry an open-core business (INFERENCE; no local market-size data found).
- Sell globally; fix the entity/payout problem first (move billing to a supported jurisdiction, use a merchant-of-record). GeniusPay/XOF is irrelevant for global revenue and only suits a small local add-on.
- Offline/low-bandwidth/local-model is worth keeping only if reframed as "air-gapped, self-hosted, data-sovereign, cheap-token" for regulated or cost-sensitive Western teams. Wedge only; a moat would need data, workflow lock-in, or a standard.
- Do not plan on grants as a business model; at best NLnet-style funding for a commons sub-component, and the NGI0 Commons call is closed.
- Better Auth shows the global path: narrow, loved, agent-friendly niche first; commercial layer second.

## Open questions

- Where will the legal entity and bank account sit, and can founders access Stripe (via Atlas), Polar, Paddle?
- Real count of francophone West African senior platform engineers and their budgets (not found).
- Is any current round of GitHub Secure OSF or Sovereign Tech open to non-EU small tools?
- Homebrew notability thresholds and MCP registry listing rules (not retrieved).
- Evidence on what drives adoption of OSS CLI tools (HN/Show HN dynamics, docs, agent skill compatibility): not found this pass.

## Three ideas to test (cheap, falsifiable)

1. **Air-gapped/local-model positioning.** Build a landing page + 2-minute demo "works with no cloud, with a local model" and post to 3 targeted communities; DM 20 platform engineers at regulated EU/US firms. Falsified if <5 of 20 reply with a real need, or <1% landing-page-to-install after ~500 visits.
2. **Entity/payments feasibility.** Within one week, attempt to open a billing account on Stripe (via Atlas or CI entity), Polar and Paddle and accept a test USD payment from a foreign card. Falsified if none can pay out to a founder-accessible account without a foreign entity; then entity relocation becomes step zero.
3. **Distribution reach without a wedge claim.** Ship the CLI to npm/pip/cargo + Homebrew tap + an MCP server entry, and measure installs and 7-day retention after one Show HN. Falsified if fewer than 50 installs with under 10% 7-day retention (agents trivially substitute it), indicating no pull beyond novelty.
