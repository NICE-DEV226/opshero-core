# Research record

This folder preserves the evidence behind docs/00-decision-record.md. Read the decision record first; come here to check a claim.

## Reports (W1–W9)

Produced on 2026-10-19 by delegated research agents using web search and the `gh` CLI, each with the same evidence rules: cite URLs, label claims FACT / REPORTED / INFERENCE, never invent numbers, look for contrary evidence.

| File | Question |
|---|---|
| W1_agent_tooling.md | Coding-agent ecosystem and what agent-side tools actually spread |
| W2_pain_evidence.md | Ranked developer/DevOps pains, paid versus loud-but-unpaid |
| W3_devops_platform_sre.md | AI SRE / DevOps / platform tooling landscape |
| W4_mlops_llmops.md | MLOps and LLMOps needs and crowding |
| W5_business_models.md | Open-source and open-core business models, what won and died |
| W6_prior_art_science.md | Prior art and research on failure diagnosis and agent tooling |
| W7_market_gtm.md | Market, payments, funding and distribution for a small francophone-West-Africa team |
| W8_candidate_scan.md | Competitive scan of five candidate product shapes |
| W9_agent_pr_outcomes.md | Measured differences between agent and human pull requests |

## How far to trust them
- Web search was US-only and often thin; many figures come from secondary sources or vendor material and are labelled.
- Several primary reports were unreachable (for example some survey pages were blocked), so their numbers are REPORTED.
- The agents summarised pages with smaller models in places; re-check a figure on its source page before quoting it.
- Star counts are an attention metric, not usage; some are visibly inflated.
- All GitHub-based measurements have a popular-repository bias.

## Experiments
- `commons-experiment/`: the pre-registered test of a cross-repository failure-signature commons. Contains the protocol and its amendments, the frozen signature and evaluation code, the blind-judge verdicts and a summary (RESULTS.md).
  - **Not included**: the corpus of 429 failure excerpts and the judging batches. They are excerpts of CI logs from third-party public repositories whose redistribution terms we did not establish. The corpus can be rebuilt by re-running the sampling described in PROTOCOL.md; it will differ because logs expire.
  - The scripts reference session-local paths and intermediate files and are provided for **provenance and review, not as a turnkey rerun**.
  - FREEZE.txt records hashes of the original frozen inputs, including files not shipped here.
- `w9-scripts/`: scripts and aggregate results for the agent-versus-human PR study. Raw per-PR data (tens of MB of third-party metadata) is excluded. The scripts contain session-local paths.

## Data that was lost
Raw data for two earlier experiments (CI failure distribution by step; the A/B test of a "what changed since last green" helper) was lost when a session scratch directory was reset. Only the conclusions survive, recorded in docs/00-decision-record.md. They should be treated as less reliable than the experiments above.
