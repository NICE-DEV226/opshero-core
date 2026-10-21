# W4 - MLOps / LLMOps: landscape, pains, crowding (2026-10-19)

Labels: FACT = primary source; REPORTED = secondary/vendor; INFERENCE = mine. Search was thin on practitioner-level evidence for training failures; no ML-engineer survey on job failures was found (unverified).

## Ranked key findings

1. **Training failures are real, frequent, and mostly hardware, but the big players already built in-house diagnosis and the open tooling is upstream.** Llama 3: 466 interruptions in a 54-day snapshot, 419 unexpected, ~78% attributed to confirmed or suspected hardware, GPU issues 58.7%; only 3 needed manual intervention; they rely on PyTorch's NCCL flight recorder for hangs (FACT, Meta paper, 16K-GPU scale, single run, not generalizable to small teams). PyTorch ships Flight Recorder + `TORCH_NCCL_*` dump-on-timeout env vars (FACT, docs). INFERENCE: the hang-diagnosis primitive exists upstream; what is missing for a normal user is probably "turn the dump into a root-cause sentence", not the capture.
2. **LLM observability/tracing is crowded and consolidating; this is the worst place to start.** Langfuse (36k stars), Phoenix (11.9k), Opik (22.7k, Apache-2.0), MLflow (28k), all pushed today (FACT, gh). Langfuse acquired by ClickHouse Jan 2026 (REPORTED, law-firm and secondary sources); W&B/CoreWeave (REPORTED, single source, unverified); Helicone/Mintlify (REPORTED, HackerNoon). MLflow already ships an experimental MCP server exposing trace search/analysis to Claude Code/Cursor (FACT, docs). INFERENCE: "agent-native CLI over traces" is already being absorbed by incumbents.
3. **Evals: observability is ubiquitous, evals lag.** LangChain survey (n=1,340, Nov-Dec 2025, 49% companies <100 staff, vendor selling observability/evals, self-selected): 89% have observability, 52.4% offline evals, 37.3% online evals; quality is top production blocker (~1/3) (REPORTED-vendor). Open CLI incumbents: promptfoo (MIT, 26k stars), deepeval (Apache-2.0, 19k) (FACT, gh). Pain points around flaky/non-deterministic eval gates are described only in practitioner blogs; no survey quantifying them (REPORTED, unverified). INFERENCE: the gap is "what to evaluate" (error analysis), not the runner. Hamel Husain's claim that ~90% of eval work is looking at data (REPORTED, secondary) suggests a human/workflow gap, which a CLI helps only partly.
4. **GPU waste is a cost story told mostly by vendors.** ClearML survey (vendor, IT leaders/enterprise): 44% assign GPUs manually or lack strategy, cost control top challenge for 53% (REPORTED-vendor). CAST AI: 5% average GPU utilization in analyzed clusters (REPORTED-vendor, non-optimized sample). Uptime Institute: training GPUs ~80% operational, 35-45% of peak compute used (REPORTED). INFERENCE: real, but buyers are platform/FinOps teams, a poor fit for an individual-senior-engineer CLI; strong crowding by Run:ai/Kubecost/CAST AI-type vendors (not verified here).
5. **Public issue trackers show diagnosis friction concentrated in env/version, hangs, and unexplained-wrong-output, and a large share lack resolution** (own study below; small n, noisy).
6. **Orchestrators/registries are mature and not where the gap is.** Kubeflow (15.9k), Flyte (7.7k), Metaflow (10.3k), KServe (6.1k), Ray (44k) all Apache-2.0 and active (FACT, gh; kubeflow/kubeflow `open_issues`=1, pushed 2026-09-30, i.e. low activity on that repo).

## Own quantitative study (gh API, public issues)
Method: for each repo, 45 random dates in 2026-01-01..09-30, then one random issue (is:issue, not PR) created that day via search API. Hit rate dropped by a secondary rate limit/TLS error, so n differs: pytorch 40, vllm 45, transformers 42, ray 39 (total 166). Bias: sampling by day over-weights low-volume days; issues include feature requests/questions (bug-labelled/titled: pytorch 1* (labels rarely "bug"), vllm 31, transformers 15, ray 17); recent issues are naturally still open; keyword classification is regex, not manual. Treat as indicative only.

| Repo (n) | Open now | Closed not_planned | Closed, 0 comments | Title-keyword OOM / hang / env-version / wrong-output |
|---|---|---|---|---|
| pytorch (40) | 12 | 1 | 5 | 0 / 0 / 3 / 3 |
| vllm (45) | 14 | 12 | 5 | 2 / 1 / 2 / 5 |
| transformers (42) | 2 | 7 | 7 | 1 / 0 / 1 / 1 |
| ray (39) | 23 | 2 | 0 closed w/ 0 (see raw) | 0 / 2 / 0 / 0 |

Body+title regex (looser, over-matches words like "install", "accuracy"): OOM 10/166 (6%), hang/NCCL/timeout 11/166 (7%), env/version 63/166 (38%), wrong-output keywords 31/166 (19%). Across all 166: 51 open (31%), 22 closed not_planned (13%) -> ~44% not resolved as a fix; ~21% (36/166 by state/zero-comment counts) got no discussion. Root-cause presence was NOT measured (would need manual reading) - open gap. INFERENCE: env/version mismatch is the most frequent recurring diagnosis class; hangs and OOM are rarer in titles but I cannot tell if they are underreported (users may debug in forums/Slack instead).

## Evidence table
| Claim | URL | Label |
|---|---|---|
| Llama 3 466 interruptions/54d, 419 unexpected, 78% hardware, flight recorder use | https://arxiv.org/abs/2407.21783 | FACT |
| Flight Recorder, watchdog, trace env vars | https://pytorch.org/blog/flight-recorder-a-new-lens-for-understanding-nccl-watchdog-timeouts/ ; https://docs.pytorch.org/docs/main/torch_nccl_environment_variables.html | FACT |
| LangChain agent survey n=1,340, obs 89%, evals 52.4%/37.3% | https://langchain.com/state-of-agent-engineering | REPORTED (vendor) |
| ClearML GPU survey | https://go.clear.ml/state-of-ai-infrastructure-report-25-26 | REPORTED (vendor) |
| GPU utilization 35-45% (Uptime) | https://theregister.com/2025/05/20/gpu_metric | REPORTED |
| MLflow MCP server for traces, works with Claude Code | https://mlflow.org/docs/latest/genai/mcp/index.html | FACT |
| Langfuse acquired by ClickHouse | https://www.orrick.com/en/news/2026/01/open-source-llm-observability-langfuse-acquired-by-clickhouse-inc | REPORTED |
| Consolidation overview (Helicone, W&B) | https://hackernoon.com/two-acquisitions-deep-two-still-standing-whats-really-happening-to-open-source-llm-observability | REPORTED, partial |
| Stars/licences/push dates (mlflow, langfuse, phoenix, opik, vllm, pytorch, transformers, ray, promptfoo, deepeval, wandb, metaflow, flyte, kubeflow, kserve) | `gh api repos/<owner>/<repo>` on 2026-10-19 | FACT (Langfuse/Phoenix/PyTorch show NOASSERTION, mixed licences; Phoenix reported Elastic 2.0) |
| promptfoo CLI-first, no tracing | https://guptadeepak.com/tools/top-7-llm-evaluation-prompt-management-platforms-2026/ | REPORTED |
| Eval flakiness/gates are blog-level | https://tianpan.co/blog/2026-04-10-non-deterministic-ci-agentic-systems | REPORTED, opinion |

## Crowding assessment
- Strong OSS exists: tracing (Langfuse, Phoenix, Opik, MLflow), eval runners (promptfoo, deepeval), experiment tracking/registry (MLflow, W&B client), orchestration, serving, hang capture (PyTorch Flight Recorder).
- Thin/unproven: cross-layer "why did this run fail/slow/diverge" synthesis (driver/CUDA/NCCL/framework version + logs + config) delivered to an agent; eval regression triage (which cases changed and why); GPU job post-mortems for small teams. Absence of evidence of tools is not evidence of demand.

## Implications for OpsHero
- INFERENCE: Do not build another tracing store or eval runner. Both sides are crowded and incumbents are adding MCP/CLI surfaces.
- The only plausible wedge is a diagnosis layer that reads artifacts others already emit (flight-recorder dumps, `nvidia-smi`/DCGM, `pip freeze`/env, torchrun logs, MLflow/Langfuse traces) and returns a ranked, evidence-linked root cause. This is still the same "log diagnosis" idea; it has the same clone-in-a-weekend risk (agents can read dumps) unless the value is accumulated knowledge (known-bad version matrices, failure signatures) - a data moat, not code.
- Env/version mismatch is the highest-frequency pain in my sample and the one most amenable to deterministic checking (compatibility matrix), which an agent cannot trivially hallucinate correctly.
- ML users run on varied hardware/clusters; trust/security (logs with secrets, proprietary data) pushes toward local-first CLI, consistent with the brief.

## Open questions
- Share of issues with a real root cause (needs manual labeling of ~200 issues).
- Do ML engineers at <500-person firms hit NCCL/OOM weekly, or is it rare outside large clusters? No survey found.
- Does an agent with Flight Recorder docs already solve hang triage? Untested.
- Willingness to pay: all evidence is vendor-sourced.

## Three product ideas with cheap falsifiable tests
1. **`env-doctor` for GPU stacks**: given a failing traceback + `pip freeze` + driver/CUDA info, check against a curated compatibility/known-bug matrix and print the fix. Test: take 50 closed env/version issues from the four repos, hide the resolution, run (a) a bare coding agent and (b) agent + matrix; falsified if (b) does not beat (a) by >=15 points of correct fix identification.
2. **Hang/NCCL post-mortem**: parse Flight Recorder dumps + torchrun logs into "rank N diverged at collective X, last good step Y". Test: reproduce 5 known hang classes with torchrun on 2 GPUs (or CPU gloo analog); falsified if a baseline agent given the raw dump reaches the right rank/collective in >=4/5 cases.
3. **Eval regression diff for agents**: compare two eval/trace runs (promptfoo/Langfuse exports), cluster changed cases and cite the likely prompt/model/tool change. Test: recruit 5 engineers with real eval suites, give 3 regressions each; falsified if triage time is not cut by >=30% vs their current dashboard, or if they say MLflow/Langfuse's own MCP already does it.
