# ADR-0001: No machine-learning model anywhere in the product

- Status: Accepted (maintainer decision)

## Context
The tool verifies claims made by AI coding agents. If a model also judged those claims, the guarantee would be circular and non-reproducible. Research also showed that LLM-based explanation of CI failures is being absorbed by platforms and is hard to differentiate (docs/research/W2, W3, W5).

## Decision
The product contains no ML model: not in the core, not as an optional feature, not for explanations. Explanations are generated from rule identifiers and structured facts.

## Consequences
- Verdicts are deterministic, auditable and reproducible byte for byte.
- No model API keys, no data leaving the machine, no per-call cost.
- Less "magic": diagnosis of novel failures is out of scope (it is also a non-goal).
- Marketing and docs can state a simple, checkable property.
- The decision concerns the **product**. Research for this project used models for blind judging; that is documented and is not part of the product. A policy on AI-assisted *contributions* is a separate question (docs/engineering-standards.md).

## Alternatives considered
- Optional LLM explanations: rejected; any model in the loop weakens the trust story and adds a dependency.
- Local small models: rejected; same circularity, plus distribution weight.
