# Naming (open decision)

Status: **undecided**. Nothing in the code has been renamed. Working name: `opshero`.

## Why consider renaming
- A PyPI project `opshero` (the previous CLI, 8 releases, latest 0.2.0) already installs an `opshero` executable. A new tool with the same binary name would collide on users' machines.
- The name carries the identity of the hosted diagnosis service that is being wound down and does not describe verification of agent work.
- Renaming is cheapest now: no published package, no users of the new tool.

## Constraints checked on 2026-10-21 (availability only; this is **not** trademark clearance)
Registries: crates.io, Homebrew, npm, PyPI. DNS: whether `<name>.dev` / `<name>.io` resolve (a missing record does not prove a domain is purchasable).
- Dictionary words (`assay`, `hallmark`, `stamp`, `tally`, `cairn`, `verity`, `sigil`, `tether`, `plumb`, `avow`, `sworn`, `evince`) are taken on at least one registry. `vouch` is also a 5,100-star trust project on GitHub.
- `aver`: crates.io name taken by a dormant logging crate (last update 2019, 17 recent downloads), npm and PyPI taken; Homebrew free; no GitHub repository of that exact name found among the top results. A binary named `aver` can ship from a crate named differently (for example `aver-cli`).
- Free on all four registries: `treeproof`, `treeseal`, `averd`, `witnesso`.

## Criteria
Describes the function or is a clean coined word; short and typeable; readable as a command (`<name> run -- pytest`, `<name> verify`); pronounceable by French and English speakers; unique enough to search for; no conflict with an existing executable or a well-known brand in developer tooling.

## Before deciding
1. Trademark and domain search in the jurisdictions that matter (the African regional IP office for the founders' region, plus WIPO Global Brand Database, EUIPO, USPTO). Not done; not legal advice.
2. Reserve the GitHub organisation, the crate name, the domain.
3. Plan the mechanical rename: binary, crates, `.opshero/` directory, `opshero.verify/v0` report schema, attestation predicate type, docs.

## Second pass: blended names (2026-10-21)
Roots: truth (aver, veri), proof, oath, witness, mark, tree (the subject is a git tree). About 25 blends were generated and checked on crates.io, Homebrew, npm, PyPI, DNS and GitHub; nearly all are free on the four registries, so meaning, sound and brand risk decide.

Eliminated after a web search:
- `veritree`: an existing funded company (nature-based solutions and tree-planting software, reported over 15 M USD raised) with a name that is also about trees. Do not use.
- `oathtree`: no product found, but one letter away from Oaktree, a very large investment firm. High confusion risk.

Finalists (none cleared for trademark; this is not legal advice):
- `avertree` (aver + tree; also reads "avert", as in averting a bad merge). No product found by the same name. Phonetically close to Avantree (consumer audio), which may matter for trademark classes that cover both electronics and software.
- `attestree` (attest + tree): explicit meaning, 9 letters.
- `witree` (witness + tree): 6 letters, ambiguous pronunciation.

## Third pass: keep the OpsHero lineage, do not name the function (2026-10-21)
Constraint given by the maintainer: the name may evolve from OpsHero and need not state "truth" or "verification".

Checked (registries and web search): `opsheron`, `opsherald`, `opsher`, `heroops`, `opsheroes`, `opsherpa`, `opshera`, `opsheros`, `opshiro`, `opsheroic` are free on crates.io, Homebrew, npm and PyPI. Web searches found no product named `opsheron` or `opsherald`.
- Eliminated: `opsero`, one letter from Opsera, a funded DevOps platform company.
- Caution: the "Ops-" prefix is crowded (Opsera, Opsware, Opsani, Opsview, OpServices, Opsgenie), which raises trademark-confusion risk and weakens search visibility.
- Leading candidate: `opsheron` (OpsHero plus "n"; the heron is a patient bird that watches still and acts only when needed; "héron" is a French word). Alternative: `opsherald` (hero plus herald, one who announces).
- Trade-off: a name this close to the legacy `opshero` signals continuity but also ties the new project's reputation to the legacy service being wound down.

## Fourth pass (2026-10-21): "ops" kept as a prefix, new metaphors

Checked: crates.io, Homebrew, npm, PyPI, DNS (.dev/.io/.com), GitHub exact-name repositories, plus a web search per finalist. Not a trademark clearance.

Free on all four registries: opstone, opskeel, opsplumb, opsgrove, opsgraft, opsanvil, opsquill, opsnotch, graftmark, keelmark, rootward, wardtree, touchops, opswright, graftops, rootops (GitHub repo with 17 stars), keelops, stoneops.
Taken on at least one registry: opsmith (npm), opsward (PyPI), opsforge (crates), stonemark and rootmark (PyPI), gatewright (npm).

Web search found no product or company named opstone, opsplumb or opsgrove (search engines return only look-alikes such as Opstarts, Plumb, Brightgrove).

Finalists: opstone (touchstone: the stone used to test the quality of metal), opsgrove / opsgraft (git trees, grafting), opsplumb (plumb line: checks that something is true to vertical).
Caveat: the "Ops-" prefix is crowded (Opsera, Opsware, Opsani, Opsview, Opsgenie), so distinctiveness rests on the suffix.

## Decision (2026-10-21): keep `opshero`

After the fifth pass the owner chose to keep **OpsHero**: the domain opshero.me is already owned, which outweighs the crowded "Ops-" prefix and the absence of a clean registry check. `asytree` (assay + git tree) was the runner-up and was briefly applied, then reverted; it remains the fallback if the name must change. Registry availability of `opshero` itself: crates.io, npm and Homebrew were not re-verified in this pass; PyPI `opshero` is already taken by the legacy CLI.
