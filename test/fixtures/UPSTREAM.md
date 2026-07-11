# Upstream Fixtures

These fixtures are copied byte-for-byte because line endings, trailing whitespace, and missing final newlines are test data.

Do not normalize or hand-edit imported fixtures.

## Project Fluent Reference

Source: `https://github.com/projectfluent/fluent/tree/3dbb402ed5af6b64f5c09faeb067acff127a0f34/test/fixtures`.

Local directory: `projectfluent-reference/`.

License: Apache-2.0.

The directory contains 39 FTL and JSON pairs used as the authoritative syntax and Junk oracle.

## Fluent Tooling Structure

Source: `https://github.com/projectfluent/fluent.js/tree/9a925d2a38b893be735ff4429be8ad62132a204d/fluent-syntax/test/fixtures_structure`.

Local directory: `fluent-tooling-structure/`.

License: Apache-2.0.

The directory contains 62 focused FTL and JSON pairs with spans, error codes, messages, and Junk boundaries.

## fluent-rs Normalized

Source: `https://github.com/projectfluent/fluent-rs/tree/b822cfe0ac5f35099ee71d3cf6f43b7c01d5fc6d/fluent-syntax/tests/fixtures/normalized`.

Local directory: `fluent-rs-normalized/`.

License: MIT OR Apache-2.0.

The directory contains seven formatter normalization and parse-serialize-parse fixtures.

## Regression Fixtures

`regressions/broken.ftl` is copied from the Probematic Fluent worktree and has SHA-256 `8fe3903b06e776f02fc15c612bf5eedec0771f450ea50c6a76b0d4902c0e0ddc`.
