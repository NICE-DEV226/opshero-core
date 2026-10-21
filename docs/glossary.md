# Glossary

- **Ambient input**: a file ignored by git, or an environment variable name, that can change a check's result; recorded as a keyed digest, never as a value.
- **Assurance level (A0–A3)**: how strongly the evidence resists a dishonest operator (RFC-0001).
- **Attestation**: a signed statement (in-toto Statement in a DSSE envelope) about a subject.
- **Check**: a named policy expectation (for example, tests ran on the final tree).
- **DSSE**: Dead Simple Signing Envelope, the signing envelope used for in-toto attestations.
- **Evidence record**: immutable record of one command execution bound to a subject state (RFC-0002).
- **Fresh / stale**: evidence is fresh for a subject when it was produced on the same content (strict) or the difference cannot affect it (scoped).
- **in-toto**: a framework and specification for software supply-chain attestations.
- **Policy**: a versioned file stating required checks, paths, minimum assurance and waivers (RFC-0004).
- **Read set**: the set of repository files a command actually read, when observed.
- **Subject**: the content being judged, identified by a git tree object id.
- **Verdict**: `PASS`, `FAIL` or `UNKNOWN`, with reasons.
