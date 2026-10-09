# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the right-hand column to match whatever vocabulary you actually use.

## Applying them here

These labels do not exist on the repository yet; `gh issue edit --add-label` fails on an
unknown label. Create them once with:

```bash
gh label create needs-triage --color FBCA04 --description "Maintainer needs to evaluate this issue" --repo PasleySage/patina
gh label create needs-info   --color D876E3 --description "Waiting on reporter for more information" --repo PasleySage/patina
gh label create ready-for-agent --color 0E8A16 --description "Fully specified, ready for an AFK agent" --repo PasleySage/patina
gh label create ready-for-human --color 1D76DB --description "Requires human implementation" --repo PasleySage/patina
gh label create wontfix       --color EDEDED --description "Will not be actioned" --repo PasleySage/patina
```

All five exist on the fork as of the initial setup, so the block above is a reference
rather than a setup step. `wontfix` was inherited from upstream together with `bug`,
`enhancement`, and the rest; its wording differs but the meaning does not, so it was left
alone rather than overwritten.

Create them on the **fork** (`PasleySage/patina`). Labels are not shared with upstream, so
triaging here never touches the original project's tracker — see
[issue-tracker.md](./issue-tracker.md).

## Upstream triage is separate

Upstream runs its own triage with its own vocabulary and its own maintainer. Its labels
are not mirrored here, and an upstream issue's state says nothing about priority in this
fork. Read upstream issues for decisions and context; track local work with the labels
above.
