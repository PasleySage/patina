# Agent skill configuration lives in docs/agents, not AGENTS.md

The engineering skills (`triage`, `to-tickets`, `to-spec`, `domain-modeling`, …) normally
scaffold their configuration by adding an `## Agent skills` block to `CLAUDE.md` or
`AGENTS.md`. This fork writes that configuration to `docs/agents/` instead and leaves
`AGENTS.md` untouched.

Two reasons, both specific to being a fork:

- `AGENTS.md` is upstream-owned (last changed by upstream commit `ecf9509`). Editing it
  turns every future rebase from `upstream` into a conflict to arbitrate over a file whose
  content this fork did not originate.
- Upstream's own rules in `AGENTS.md` ("Optional Local Agent Skills") and
  `docs/engineering-quality.md` state that local agent tooling and its routing must not
  become repository instructions or long-lived documentation. A block naming specific
  tools and pointing at them is exactly what those rules exclude.

The cost is that a skill reading `AGENTS.md` for its entry point will not find one. The
configuration is still discoverable at the conventional location, and this file is the
pointer for anyone who wonders why `AGENTS.md` looks untouched.

**Consequences**: if upstream ever adds its own `## Agent skills` block, this ADR should
be revisited rather than assumed to still hold — the conflict cost would be real but so
would the consistency gain. Upstream's `AGENTS.md` remains authoritative for how the
project is built; nothing in `docs/agents/` overrides it.
