# W1 - AI coding-agent ecosystem and the "tools FOR agents" layer (as of 2026-10-19)

Labels: FACT = primary source; REPORTED = secondary/vendor; INFERENCE = mine. Many fetches were summarized by a small model; numbers worth quoting should be re-checked on the page. Stars are an attention metric, not usage (see caveat F2).

## Ranked key findings

1. **Claude Code and Copilot lead agent use; terminal-agent usage is mainstream.** FACT (SO 2026, n>30,000; one chart n=16,664): of AI-tool users, 66% work in coding agents; Claude Code 66% and Copilot 59% are the most-used agents; 73% of assistant/agent users use them daily. Only 20% use AI for deploy/operate/troubleshoot of production. Cursor/Codex/Gemini CLI shares and MCP usage were NOT in the article (unverified). The 2026 SO survey is self-selected (SO audience). Ops/prod use being only 20% is a signal that the ops side of agents is under-served or under-trusted (INFERENCE).
2. **The only tool-layer products with huge traction are context/token reducers and agent "infrastructure", and the winners are single-binary/hook-based.** rtk (CLI proxy, Rust single binary, PreToolUse hook rewriting `git status` to `rtk git status`) has 83.9k stars, 9 months old (FACT via gh); the 60-90% savings claim is self-reported, no independent benchmark (README, FACT that it is a claim). context7 63k stars, 49 open issues (FACT). context-mode 31k stars in 8 months (FACT). Common traits: measurable promise (tokens saved, `rtk gain` dashboard), install in 1 command, plugs into an existing integration point (Bash hook / MCP), no workflow change. INFERENCE.
3. **Context cost is the most documented, quantified complaint, and platforms are already absorbing it.** REPORTED (vendor blogs, Scalekit/Apideck): MCP tool definitions can cost ~55k tokens for ~40 tools; one benchmark claims MCP 4-32x more tokens than CLI. Claude Code shipped MCP Tool Search lazy loading (v2.1.7, per secondary blogs; official announcement not found). Risk for any "token saver" pitch: native fixes erode it (INFERENCE). rtk itself only covers Bash tool output, not Read/Grep/Glob (FACT, README).
4. **Verification/review is the strongest unmet pain with numbers, but evidence is vendor-heavy.** REPORTED (Sonar, vendor, n>1,100): 96% do not fully trust AI code; only 48% always verify before commit; 38% say AI code review is more work than human code. FACT (SO 2025/2026): trust is conditional: 48% trust AI when output is easy to verify; 6.6% trust it for important decisions; 66% (2025) complain "almost right, but not quite". Qodo/LinearB claims of review bottleneck (PR size 44 to 72 lines median) are secondary and vendor-sourced. Implication: value goes to tools that make output cheaply verifiable (INFERENCE).
5. **Permissions/safety and security of the tool layer itself are real gaps with low-quality incumbents.** FACT: Anthropic ships native sandboxing (OS-level fs+net isolation) and hooks. REPORTED: community uses PreToolUse hooks to block `rm -rf`; reports of Claude resuming edits after compaction against CLAUDE.md rules (single GitHub issue, not a proven pattern). MCP security: 30+ CVEs in early 2026, Endor Labs survey of 2,614 servers (82% path-traversal-prone) per vendor-ish blogs; postmark-mcp backdoor (Sep 2025). Trust in third-party MCP/skills is a genuine barrier, and a trusted, audited, single-binary tool can differentiate (INFERENCE).
6. **Cost/usage visibility and session capture have a lively long tail with no dominant vendor.** FACT (gh): codeburn (token/cost tracker across 37 agents) 11.5k stars MIT; entireio/cli (captures agent sessions alongside commits) 5.2k stars, 343 open issues. Memory/context: context-mode, engram 7.4k, mem0 67k (general). No clear owner for cross-agent "what did the agent do and why" (INFERENCE).
7. **Discovery is fragmented and weakly curating.** REPORTED: official MCP Registry (preview, ~2,000 servers; repo has 7.4k stars), Smithery (3k-7k claimed), Glama (20k+), mcp.so (~19.7k); counts inconsistent, registries verify namespace ownership, not security or quality. GitHub awesome-mcp-servers 96.6k stars. Winners seem to grow by GitHub/X/Reddit virality and by being listed in agent docs, not by registry rank (INFERENCE, no direct evidence of registry-driven installs; unverified).
8. **Productivity evidence is mixed, so "measurable promise" must be local.** FACT (METR 2025, arXiv 2507.09089): 16 experienced devs, 246 tasks, 19% slower while believing 20% faster. REPORTED (one blog): a 2026 follow-up shows ~18% speedup with wide CI; I could not find the primary METR post (their 2026 post list has no such item). Do not rely on productivity claims either way.

Caveat F2 (star inflation): affaan-m/ECC has 283.7k stars and obra/superpowers 302k stars, anomalously large against issue counts (246 / 383) and for config/skill collections. rtk has 5.3k forks. Stars here likely reflect hype/starring-bots/one-click-promos as much as use; no install telemetry found (INFERENCE). Agents' own star counts: opencode 217k, claude-code 152k, codex 130k, gemini-cli 107k, goose 55k, cline 70k, aider 49.6k (last push 2026-05-22, apparently stalled) (FACT, gh).

## Evidence table

| Claim | Source URL | Label |
|---|---|---|
| SO 2026: 66% of AI users in coding agents; Claude Code 66%, Copilot 59%; 20% for prod ops; n>30k | https://stackoverflow.blog/2026/10/06/the-results-of-the-2026-developer-survey-are-here/ | FACT |
| SO 2026 trust: 48% trust if easy to verify; 6.6% for important work | https://itbrief.com.au/story/stack-overflow-survey-shows-developers-trust-ai-with-caveats | REPORTED |
| SO 2025: 31% use agents; 66% "almost right"; 87% accuracy concern | https://shiftmag.dev/stack-overflow-survey-2025-ai-5653/ | REPORTED |
| rtk: 60-90% claim, hook, single Rust binary, Bash-only | https://github.com/rtk-ai/rtk | FACT (claim self-reported) |
| Repo stats (stars/license/push) | `gh api repos/{rtk-ai/rtk, upstash/context7, mksglu/context-mode, getagentseal/codeburn, entireio/cli, affaan-m/ECC, anomalyco/opencode, openai/codex, google-gemini/gemini-cli, anthropics/claude-code}` 2026-10-19 | FACT |
| MCP token cost 55k / 4-32x | https://www.apideck.com/blog/mcp-server-eating-context-window-cli-alternative | REPORTED (vendor selling CLI) |
| Tool Search lazy-loading in Claude Code | https://claudelog.com/faqs/what-is-tool-search-in-claude-code/ | REPORTED |
| Claude Code sandboxing | https://docs.claude.com/en/docs/claude-code/sandboxing | FACT |
| Sonar: 96% distrust, 48% always verify, n>1,100 | https://www.sonarsource.com/company/press-releases/sonar-data-reveals-critical-verification-gap-in-ai-coding/ | REPORTED (vendor) |
| Semgrep: warns against vendor review figures | https://semgrep.dev/blog/2026/end-is-nigh-for-code-review | REPORTED (vendor) |
| MCP CVEs / Endor Labs 2,614 servers | https://mcp.directory/blog/mcp-security-200000-exposed-servers-owasp-mcp-top-10-cves | REPORTED |
| Tool-poisoning metadata attack (paper) | https://arxiv.org/pdf/2607.05744 | FACT (preprint, unreviewed) |
| Registry landscape | https://codex.danielvaughan.com/2026/05/27/codex-cli-mcp-server-discovery-registries-smithery-glama-official-registry/ | REPORTED |
| METR 2025 RCT n=16 | https://arxiv.org/abs/2507.09089 | FACT |
| Claude Code run-rate ~$2.5B Feb 2026 | https://www.gradually.ai/en/claude-code-statistics/ | REPORTED (unaudited) |
| Compaction/permission complaints | https://claudeissues.com/issue/61843-claude-resumes-work-automatically-after-context-compaction-overriding-users-expl | REPORTED (single issue) |

## Shared traits of tools that really spread (INFERENCE from F2-F6)
Measurable immediate promise (tokens, cost, docs freshness); one-line install; sits at an existing hook/MCP integration point; works across many agents (rtk: 18 agents, context-mode: 17 platforms); local-first, no account; MIT/Apache. Counter-evidence: all are cloneable in a weekend, same as the CI idea, and rtk shows the open-core split (Apache + BUSL engine editions) in the README, so the moat is distribution, not code.

## Gaps without a strong incumbent (INFERENCE, not proven absent)
- Agent-output verification that is cheap and deterministic (what actually ran, what was not tested, diff vs. claims).
- Cross-agent action audit/permission policy as code (hooks exist, but only DIY snippets found).
- Trusted/audited MCP+skill supply chain (scanner, pinning, signed provenance).
- Ops/prod-safe agent access (read-only, scoped, redacted); only 20% use agents for prod ops (FACT), maybe for lack of safe tooling.
- Cross-agent cost/context attribution beyond dashboards (codeburn is trackers, not controllers).

## Implications for OpsHero
- CI diagnosis as a standalone product is a poor fit: the agent loop already reads logs; the adopted tools all reduce cost or add safety at an existing integration point. A "diagnosis" benefit is hard to measure locally; a "tokens/time/risk saved" benefit is.
- The measurable-promise pattern suggests framing OpsHero as an agent-side ops layer with a built-in meter (like `rtk gain`): e.g. compress CI/infra/log/kubectl/terraform output (ops commands are exactly where output is huge) and make agent access to ops systems safe. Note rtk already covers many of these commands, so differentiation must come from semantics (failure-aware, not generic truncation), which is untested.
- Platform absorption risk is high for pure context-reduction (Tool Search precedent); safety/audit with cross-vendor neutrality is less likely to be absorbed by one vendor (INFERENCE).

## Open questions
- Real install counts (npm/brew/crates downloads) for rtk, context7; stars may be inflated.
- Do experienced seniors actually run rtk-like proxies, or is it vibe-coder driven? No demographic split found.
- Cursor/Codex/Gemini CLI usage shares and MCP usage: not found in primary sources.
- Does token saving change outcomes (accuracy) or only cost? No independent benchmark found.
- Whether registries drive installs at all.

## 3 product ideas worth testing (each with a cheap falsifiable test)
1. **Ops-output compressor with failure-aware semantics** (terraform plan, kubectl, docker/CI logs, pytest/cargo failures) as a hook binary with a savings meter.
   Test: on 30 recorded real failing sessions, compare tokens and agent fix success (same agent/model) with rtk vs. OpsHero filter. Falsified if tokens saved <20% beyond rtk, or success rate is not >= baseline.
2. **Agent action policy + audit (policy-as-code hooks)**: declarative allow/deny/confirm for shell/ops commands, secret redaction, append-only action log, "explain what the agent did" report.
   Test: ship a 1-file policy hook to 10 senior users for 2 weeks; falsified if fewer than 5 keep it enabled after week 2 or it blocks <1 genuinely risky action per user.
3. **Verification receipt for agent PRs**: a CLI that records which tests/commands the agent really ran and produces a diff-vs-claims receipt for the reviewer.
   Test: give 5 reviewers 20 agent PRs with and without receipts; falsified if review time or missed-defect rate does not improve by >=20% (blind comparison). Related incumbent to check first: entireio/cli (session capture).
