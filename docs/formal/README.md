# Formal models

TLA+ models checked with TLC (tla2tools 1.8.0, Java 27). They are specifications of the protocol, not generated from the Rust code; the Rust tests in `crates/opshero-core/tests/verdict_laws.rs` check the code against the same properties.

## Freshness.tla

Question: can the verifier answer PASS for the current content on the strength of a run during which the content changed?

State: the current content `wt`, the run in progress (content measured before it started, number of edits since), and the set of records (`before`, `after`, `pass`, plus a ghost field `stable` that the verifier cannot see).

Property `Sound`: a record accepted for the current content has `stable = TRUE`. Property `Complete`: a stable passing run on the current content is always accepted.

| Config | Rule | Edits per run | Result |
|---|---|---|---|
| `Freshness_allow_edits1.cfg` | `allow` (the former v0 default, now an explicit opt-out: count a run when tree after = current) | 1 | **Sound violated**, 4 states: start a run on content 1, edit to content 0 while it runs, finish with a pass: PASS for content 0 on a run that began on 1 |
| `Freshness_strict_edits1.cfg` | `deny` (also require tree before = tree after) | 1 | No error; 687 distinct states, exhaustive; `Complete` holds |
| `Freshness_strict_edits2.cfg` | `deny` | 2 | **Sound violated**, 5 states: ABA, content goes 1, 0, 1 during the run and ends equal to where it began |

Reading: the rule `deny` (policy field `post_run_changes`, the default since 2026-10-21) closes the common case, a single concurrent change. It cannot close ABA: a local recorder that only compares two measurements cannot see a change that reverts itself. Closing it needs evidence from a party that controls the run (assurance A2/A3: the CI job executes the check itself on a fixed snapshot), which is the reason the assurance levels exist. This is a limit of local evidence, not a bug to fix.

## Reproduce

```
curl -L -o tla2tools.jar https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar
cd docs/formal
for c in Freshness_allow_edits1 Freshness_strict_edits1 Freshness_strict_edits2; do
  java -XX:+UseParallelGC -cp tla2tools.jar tlc2.TLC -config $c.cfg Freshness.tla
done
```

Expected: the first and third configurations report `Invariant Sound is violated` with the traces above; the second reports `No error has been found`. The violations are the point of those two configurations (they document the limits); do not "fix" them by changing the expectation.

Not modelled yet: hash-chain forks under concurrent recorders (benign for the verdict by L1 and L6 but unproven here), transport of evidence from local to CI.
