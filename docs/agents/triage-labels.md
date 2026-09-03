# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

## Dispatch states

Beyond triage, `.github/workflows/agent-dispatch.yml` owns these. Set them through the workflow, not by hand.

| Label             | Meaning                                                              |
| ----------------- | -------------------------------------------------------------------- |
| `agent-running`   | Dispatched to a Cursor Cloud Agent; the agent id is in a comment       |
| `agent-failed`    | The agent ended without opening a PR; needs a human look               |
| `epic`            | A container (typically a `/to-spec` spec). **Never dispatched.**       |
| `needs-macos`     | Touches Swift/Xcode. Cloud agents run Linux and cannot build it.       |
| `needs-research`  | Requires grilling + reference research before it can spawn slices      |

`ready-for-agent` is the dispatch trigger. `/to-spec` publishes its spec issue with that label, so **relabel a spec `epic`** — otherwise an agent tries to build the whole spec in one run.
