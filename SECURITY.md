# Security Policy

## Supported Versions

Only the latest release receives security updates.

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| older   | :x:                |

## Reporting a Vulnerability

Please report security vulnerabilities privately by emailing [security@x-cmd.com](mailto:security@x-cmd.com).

Do not open a public issue for security problems. We will acknowledge receipt within 5 business days, provide a timeline for a fix within 14 business days, and coordinate a disclosure date with you.

## Continuous fuzzing

The conversion core is exercised by libFuzzer harnesses under [`fuzz/fuzz_targets/`](fuzz/fuzz_targets/), run nightly on every push through the [Fuzz workflow](https://github.com/ljh-sh/zhhz/actions/workflows/fuzz.yml) under three sanitizers (`address`, `memory`, `undefined`). Crash artifacts are uploaded to the workflow run for triage.

The project is also queued for inclusion in [google/oss-fuzz](https://github.com/google/oss-fuzz); once accepted, bugs found by ClusterFuzz will be reported here as GitHub issues.
