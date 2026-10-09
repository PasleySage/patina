# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`CONTEXT.md`** at the repo root, or
- **`CONTEXT-MAP.md`** at the repo root if it exists: it points at one `CONTEXT.md` per context. Read each one relevant to the topic.
- **`docs/adr/`**: read ADRs that touch the area you're about to work in. In multi-context repos, also check `src/<context>/docs/adr/` for context-scoped decisions.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

This repo is **single-context**:

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-chime-hangs-off-tool-alert-path.md
│   └── ...
└── src/
```

`pnpm-workspace.yaml` exists at the root, but it declares no `workspaces` and there is no
`packages/*` — the frontend, `src-tauri`, and the Rust crate form one product with one
glossary, so there is no `CONTEXT-MAP.md`. Introducing a second context needs a real
boundary between independently-owned vocabularies, not a build-tooling detail.

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal: either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders), but worth reopening because…_

## Relationship to upstream's own documentation

Upstream ships long-lived documents (`AGENTS.md`, `docs/architecture.md`,
`docs/engineering-quality.md`, `docs/product-principles-and-scope.md`, and others) that
govern how the project is built and what it may become. Those remain authoritative for
this fork: `AGENTS.md` is applied in every task, and a change that conflicts with it needs
an explicit decision rather than a local preference.

`CONTEXT.md` and `docs/adr/` sit alongside them with a narrower job: the glossary names
concepts, and an ADR records a decision this fork made and why. They do not restate or
override the upstream rules — when the two disagree, upstream's own rules describe the
project, and a local ADR explains what was decided here.

Because `AGENTS.md` is upstream-owned and this fork rebases from `upstream`, prefer adding
local notes in these files over editing it. A local edit there becomes a conflict to
arbitrate on every sync.
