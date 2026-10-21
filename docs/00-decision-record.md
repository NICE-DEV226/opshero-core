# Decision record: why a verifier of agent work

Audience: maintainers and contributors who need to know how we reached this direction, what was refuted, and what is still unproven. Evidence files are in docs/research/. Each statement carries its label: **FACT** (primary source or our own measurement), **REPORTED** (secondary or vendor source), **INFERENCE** (our reasoning).

## 1. Starting point
A previous product (OpsHero) was a hosted service diagnosing CI/CD failures with a regex engine plus an LLM fallback, a web dashboard, an admin panel and a CLI. The hosted service is being wound down. The team decided to pivot to an open-source, open-core tool for experienced engineers with minimal web surface and **no LLM** (ADR-0001).

## 2. What the research established
- **CI failure diagnosis is not where the money is.** FACT/REPORTED: money flows to CI speed and cost (runner vendors), and platforms absorb explanation features (W2, W3). AI incident-analysis is crowded and prices are collapsing (W3).
- **Thin local CLIs rarely monetize; hosted state does.** FACT: Earthly shut down in 2025 citing non-converting bottom-up adoption; Sentry, PostHog, Supabase, Grafana monetize hosted state; Bun joined Anthropic after reporting $0 revenue, and Astral announced it would join OpenAI (closing not confirmed in the sources found) — exits to strategic buyers rather than revenue-based outcomes (W5). A single-binary agent tool (RTK) has very high star counts with no visible revenue (W1, W5; stars are an attention metric).
- **The best-evidenced AI-era pain is verification** (W1, W2): "almost right" output (Stack Overflow 2025, FACT), conditional trust, a verification gap (Sonar, vendor-reported). The paid slot for generic review is crowded.
- **Agent governance and generic memory are crowded** (W8): first-party sandboxes and gateways from large vendors; generic memory projects with tens of thousands of stars.
- **Evidence that deterministic structured tools beat a shell-capable agent is weak or absent** (W6): bash-only agents match or beat typed tool catalogs on the benchmarks found; no study showed a deterministic diagnosis tool beating such an agent on CI failures. Value must come from information an agent cannot cheaply derive.

## 3. What we measured ourselves
- **Where CI jobs fail (public repos, 47 popular repositories, heuristic by step name):** about 62% of failed jobs fail in test steps and at most about 13% of runs fail purely before project code. The "environment explains most failures" hypothesis was **not supported**. *The raw data was lost when a session scratch directory was reset; the numbers are recorded here as reported at the time and cannot be re-derived without re-running the method.*
- **Agent versus human pull requests (W9, 5,532 PRs in 220 repositories):** same-repository differences are mostly null; most agent PRs live in tiny repositories; CI coverage in the dataset was too thin (about 758 of 5,532 PRs) to support CI-outcome claims.
- **A/B test of a "what changed since last green" helper for agents (6 seeded faults, 24 runs):** **null result**; both arms were about 100% correct on easy cases. *Raw data lost; conclusion recorded.*
- **Public failure-signature commons (pre-registered, docs/research/commons-experiment):** estimated valid cross-repository match rate of 1.5–1.9% for the pre-registered signature (threshold 15%): **not supported**. Exploratory fuzzy matching found 15.8% same-class matches (lower bound) but only 3.8% with the same fix; what transfers is generic "head" knowledge. Within-repository recurrence was about 63%, inflated by chronically red workflows.

## 4. Options considered
| Option | Verdict | Main reason |
|---|---|---|
| CI log explanation (LLM or regex) | Rejected | Absorbed by platforms; not paid; clonable |
| Public failure-signature registry | Rejected for now | Experiment: long tail, repo-specific; transferable part is common knowledge |
| "What changed since last green" tool | Rejected as a product | Free popular tools exist; null A/B on easy cases |
| Agent governance / sandboxing | Rejected | Crowded and absorbed by large vendors |
| Generic agent memory | Rejected | Saturated |
| Output compressor for agents | Rejected | Platforms absorb it; no revenue evidence |
| Dependency verification as the product | Rejected as a product, kept as a **policy rule** | Strong free incumbents (OSV-Scanner, Dependency Review, Scorecard); the slopsquatting risk is real (reported studies) but narrow |
| **Verifier of agent work (this project)** | **Chosen, unproven** | Only direction combining documented pain, no direct deterministic incumbent found, and a hosted-state revenue path |
| Private per-repository memory | Fallback, unproven | High within-repo recurrence but inflated; competes with forge-native memory |

## 5. Why this direction, honestly
Reasons: documented pain; an unoccupied niche (the nearest tool we found, entireio/cli, captures agent sessions for traceability but does not verify claims); a monetization shape that matches winners (open standard and local tool, paid hosted state).

Weaknesses we accept openly:
- The core is easy to clone; protection must come from the open format with a conformance suite, adoption in agent hooks, and the hosted layer.
- Forges and agent vendors may build this natively.
- **Demand is unproven.** A check of public GitHub (docs/validation-plan.md, H0) found the need is felt (issue reports, at least 14 hobby projects building stop hooks and attestations, the largest with 40 stars) but with no project holding real traction, which is consistent with either an open niche or little demand. Those projects are also the closest competitors. Much of the supporting data is vendor-sourced. The pain cited is about verification in general, not about this exact tool. docs/validation-plan.md pre-commits the results that would make us stop.

## 6. Limitations of the research method
- Web search was US-only and thin; many numbers come from secondary sources and are labelled.
- Research reports were produced by delegated agents using web search and `gh`; they were not independently re-verified except where stated.
- LLM judges were used for **research** (blind judging of log pairs); the product uses none.
- Popular-repository bias in all GitHub-based measurements.
- Some earlier raw datasets were lost (see §3).

## 7. Mistakes we made along the way (kept for transparency)
- We first believed environment diffs would explain most CI failures; our own measurement refuted it.
- A tooling bug (the GitHub CLI refusing logs containing terminal escape sequences) would have biased a sample toward uncolored logs; it was caught and fixed before analysis, and logged in the experiment protocol.
- We repeatedly drifted back to "CI log analysis" after being told to widen the scope.
