# W9 - Agent-authored vs human PRs in the same repositories (2026-10-19)

Window: PRs created 2026-07-06..2026-09-13. Raw tables: `W9/results.md`, `W9/tables/pr_rows.csv`; commands: `W9/README.md`. CIs: Wilson (proportions), bootstrap (medians), cluster bootstrap over repos for repo-matched differences.

## Headline and a hard warning
**CI-verdict coverage is only partial.** The CI-enrichment step (fetch_ci.py) was stopped early at the coordinator's request: 62 of 220 repos done. Check-suite verdicts exist for ~620 of 5,532 PRs (agent apps n=165, Claude-marker n=140, other-marked n=15, human n=266). **First-run CI, pushes-before-green, time-to-green and 7-day follow-up/revert claims are NOT supported** (the tables show them only for transparency; the subset is non-random and dominated by repos that ran CI). Outcome (merged/closed/open), time-to-merge, file-touch and review-comment metrics use all 5,532 PRs (copilot 606, devin 491, cursor app 415, claude app 418, Claude-marker 1,448, other-marked 215, human 1,939; 220 repos).

## How agents were identified
| group | signature | n PRs |
|---|---|---|
| copilot | author `copilot-swe-agent` | 606 |
| devin | author `devin-ai-integration[bot]` | 491 |
| cursor_app | author `cursor[bot]` | 415 |
| claude_app | author `claude[bot]` (Claude GitHub app) | 418 |
| claude_marker | human account; body "Generated with Claude Code" or a commit with `Co-Authored-By: Claude` | 1,448 |
| other_marked | human account; head branch `codex/*`, `cursor/*`, or "Cursor Agent" in commit | 215 |
| human | no marker above, non-bot (contaminated by unmarked agent use) | 1,939 |

Codex has no app author in search; only branch-prefix matches were found. Jules/Codegen were not searched (rate limit).

## Sample and star composition
Discovery returned exactly 400 PRs per signature (cap hit: weekly slices, 40 each, GitHub "best match" order, so not a random sample). Share of discovered agent PRs in repos with >100 stars (before my repo subsampling): copilot 26/400 = 6.5%, devin 12/400 = 3%, claude app 23/400 = 6%, Claude-marker 57/400 = 14%, cursor 0/400 (all 1,084 discovered repos: only 79 have >100 stars). Of the sampled agent-app PRs, 79% are in repos with <=10 stars (median stars 0). Because I forced all 79 big repos into the sample, the in-sample table below over-represents big repos; the discovery figures above are the honest "where agent PRs live" numbers: **the large majority of agent PRs found are in tiny personal repos**.

| group (in sample) | n | >100 stars (sample) |
|---|---|---|
| copilot | 606 | 21% [18-24] |
| devin | 491 | 23% [20-27] |
| cursor_app | 415 | 0.2% |
| claude_app | 418 | 31% [27-36] |
| claude_marker | 1,448 | 47% [45-50] |
| human | 1,939 | 80% [78-82] |

## Outcomes, all repos (n PR; % [Wilson 95%])
| group | merged | closed-unmerged | open | median time-to-merge h [boot CI] |
|---|---|---|---|---|
| copilot | 81% [77-84] | 16% [13-19] | 3% [2-5] | 0.2 [0.2-0.3] |
| devin | 84% [80-87] | 4% [3-7] | 12% [9-15] | 0.2 [0.2-0.3] |
| cursor_app | 13% [10-16] | 8% [6-11] | 79% [75-83] | 3.6 [1.2-10.6] |
| claude_app | 80% [76-83] | 5% [4-8] | 15% [12-19] | 0.9 [0.5-1.5] |
| claude_marker | 71% [69-74] | 8% [7-10] | 20% [18-22] | 0.7 [0.6-0.8] |
| human | 61% [59-63] | 16% [14-18] | 23% [21-25] | 5.8 [4.1-9.7] |

Pooled numbers are confounded by repo type: most agent PRs sit in solo repos where the owner self-merges within minutes (and cursor PRs mostly stay open, apparently untouched), while the human pool is 80% >100-star repos.

## >100-star repos only (the comparison that matters)
| group | n (repos) | merged | closed-unmerged | open | median TTM h | review thread >=1 | touches test files |
|---|---|---|---|---|---|---|---|
| copilot | 126 (18) | 83% [75-88] | 13% [9-21] | 4% [2-9] | 1.4 [1.1-2.3] | 38% [30-47] | 46% [38-55] |
| devin | 114 (7) | 74% [65-81] | 12% [7-20] | 14% [9-22] | 3.4 [0.8-8.4] | 47% [38-56] | 75% [66-82] |
| claude_app | 131 (14) | 76% [68-83] | 10% [6-16] | 14% [9-21] | 19.9 [8.1-47.9] | 24% [18-32] | 64% [56-72] |
| claude_marker | 685 (63) | 57% [54-61] | 16% [13-19] | 27% [23-30] | 4.6 [2.8-12.4] | 31% [27-34] | 48% [44-51] |
| human | 1,549 (76) | 54% [52-57] | 18% [16-20] | 27% [25-30] | 20.2 [17.7-23.4] | 34% [32-37] | 44% [42-47] |
| cursor_app | 1 | not analysable | | | | | |

Merge rates are similar to or slightly above human rates; closed-unmerged is not higher for agents. Note the human pool is the 40 newest PRs per repo (later in the window), so its open share is time-biased; and agent merges can be quick self-merges.

## Repo-matched differences (agent minus human, pp; equal weight per repo, repos having both; cluster-bootstrap 95% CI)
| group | merged | touches test | touches CI/test/cfg | review thread >=1 | size >200 lines |
|---|---|---|---|---|---|
| copilot (35 repos) | -7.1 [-19.7, +5.0] | -6.0 [-22.3, +9.3] | -6.7 [-22.6, +9.6] | -7.1 [-18.4, +3.3] | -13.1 [-28.4, +1.8] |
| devin (10) | +5.3 [-21.0, +31.4] | -10.8 [-30.9, +4.2] | -11.2 [-33.3, +4.7] | -20.9 [-44.3, +0.1] | -24.1 [-50.0, -0.7] |
| cursor_app (8) | -23.8 [-56.2, +4.3] | +13.6 [-1.0, +27.8] | -5.2 [-36.7, +19.0] | -14.8 [-45.7, +13.2] | -13.7 [-41.0, +10.1] |
| claude_app (26) | +12.3 [-4.1, +26.8] | **+24.4 [+8.6, +41.2]** | +7.9 [-7.4, +25.1] | -8.2 [-23.3, +5.1] | +16.4 [-1.3, +34.2] |
| claude_marker (90) | -0.8 [-7.8, +6.3] | +2.2 [-4.0, +8.8] | +1.0 [-5.4, +7.3] | +3.7 [-1.1, +8.9] | +4.7 [-2.6, +12.0] |
| all 4 bots (76) | +0.5 [-8.9, +9.3] | +4.8 [-5.5, +14.9] | -2.8 [-12.9, +7.3] | **-10.4 [-18.4, -3.1]** | -5.3 [-15.5, +5.6] |

Within the same repos, the dominant result is **null**: merge rate, test/CI/config touching and size do not clearly differ. Two intervals exclude zero (claude app touches tests more, +24 pp; bot PRs get fewer review threads, -10 pp) among ~30 comparisons, so treat them as hypotheses, not findings (no multiplicity correction). Share touching CI/test/config is ~55-75% for every group (including humans, 57%): the regexes are broad.

## Not supported by this data
- (b) first-run CI conclusion, pushes before first green, (c) time-to-first-green, (d) follow-up fix/revert within 7 days: only ~620 PRs have verdicts. Descriptive subset only: first-run fail among PRs with a verdict was 21% [16-28] (n=165) for agent apps vs 39% [33-45] (n=266) for human, but this is NOT repo-matched in a reliable way (matched: -18.8 pp [-33.7, -5.4], 19 repos) and is subject to selection. Median pushes before first green was 0 for every group. 7-day follow-up fix/revert counts are tiny (agent apps 5/115, human 7/192; CIs overlap fully). Do not cite.
- Copilot first-run runs needing maintainer approval ("action_required") were rare in this subset (0), which may reflect the subset rather than reality.

## What this implies for a verification/diagnosis tool for agent workflows
1. We found no evidence that agent PRs fail more, are rejected more, or need more review than human PRs in the same repos. The "agents create a CI/review burden crisis" claim is not supported by outcome data at this granularity; neither is its opposite.
2. Where agent PRs live is the stronger signal: 93-97% of discovered bot-agent PRs are in repos with <=100 stars, usually single-owner repos where review barely exists (review threads 3-11% for cursor/claude apps in small repos). Experienced-engineer teams (the target) are a thin slice of the agent PR volume visible publicly; this is a market-size warning for the open-core CLI.
3. Within >100-star repos, bot PRs get review at human-like rates (24-47% with >=1 thread), so a review/verification layer matters there; the gap to measure next is CI cost per PR and diagnosis time, which this study could not complete.
4. Recommended follow-up (small): finish CI enrichment for the 220 repos (~1 hour, rate-limit gated), restricted to >100-star repos.

## Limitations
- CI data coverage ~11% of PRs (stopped early); first-run CI claims unsupported.
- Discovery capped at 40 PRs/week/signature in "best match" order; not random. Repo subsample: all 79 big repos plus 160 random small repos out of 1,084.
- Human pool: 40 newest PRs per repo in window, up to 12 per repo for CI; later-window skew inflates open share and shortens observed ages. Large repos (thousands of PRs) are thinly sampled.
- Human PRs include unmarked agent use (contamination, biases toward null); Claude-marker PRs are human-opened and measure human+agent workflows. Codex/Jules/others essentially not identified.
- Survivorship fixed for state (closed/open included) but time-to-merge is conditional on merge; open PRs are right-censored.
- File lists truncated at 50 (346 PRs with >50 files); touch metrics are lower bounds there. Path regexes are broad.
- Clustering: Wilson CIs treat PRs as independent though they cluster in repos (too narrow); only matched differences use repo-level bootstrap.
- Self-merge in solo repos makes merge/time-to-merge not comparable to team review.
- ~30 uncorrected comparisons; no causal claims.
