# Domain Docs

This repository uses a single-context domain-documentation layout.

## Before exploring, read these

- `CONTEXT.md` at the repository root.
- Relevant ADRs under `docs/adr/`.

If these files do not exist, proceed silently. Skill(domain-modeling) creates them lazily when terminology or architectural decisions are resolved.

## File structure

```text
/
├── CONTEXT.md
├── docs/adr/
└── src/
```

## Use the glossary's vocabulary

When output names a domain concept, use the term defined in `CONTEXT.md`. Do not drift to synonyms the glossary explicitly avoids.

If a required concept is absent, reconsider whether the term belongs to the project or note the gap for Skill(domain-modeling).

## Domain language and API identifiers

Use glossary terms in prose and design discussions. Preserve exact identifiers in code, queries, and API documentation even when their spelling differs from the glossary.

The domain terms **FTL resource** and **resource comment** correspond to the current Tree-sitter identifiers `fluent_file` and `file_comment`. Treat any proposal to align those identifiers as an explicit API change, not an automatic terminology cleanup.

## Flag ADR conflicts

If proposed work contradicts an existing ADR, surface the conflict explicitly rather than silently overriding the decision.
