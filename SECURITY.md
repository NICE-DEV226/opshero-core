# Security policy

## Reporting a vulnerability

Please **do not open a public issue** for a security problem. Use GitHub's private vulnerability reporting: open the repository's **Security** tab and choose **Report a vulnerability**. The report stays private between you and the maintainers until a fix is released.

We aim to acknowledge a report within 72 hours and to agree on a disclosure date with you.

## Scope and honesty about what the tool guarantees

opshero verifies that checks ran on exact content. Its guarantees depend on the assurance level of the evidence (docs/rfc/0001): locally recorded evidence (A1) is advisory and cannot resist a dishonest operator; evidence produced by the CI itself (A2, `opshero ci run`) trusts the CI configuration and the test commands. Reports that show a way to make a verdict PASS when the documented guarantees say it must not are in scope and welcome, including parsing of hostile input (policy files, evidence records, hook payloads) and the evidence transport.

## Supported versions

The project is pre-release (0.0.x). Only the latest commit on the default branch is supported.
