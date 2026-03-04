# Session Lifecycle

Every agent session in Ennio progresses through a state machine with 16 states.

## State Machine

```
Spawning → Working → PrDraft → PrOpen ──→ CiPassing → ReviewPending → Approved → Merged → Done
                                  │            │             │             │
                                  │        CiFailed    ChangesRequested   MergeConflicts
                                  │            │             │             │
                                  │       CiFixSent    (agent fixes)   (agent rebases)
                                  │            │
                                  │       CiFixFailed
                                  │
                                  ├── Exited (restorable)
                                  └── Killed (terminal)
```

## States

| State | Terminal | Description |
|-------|----------|-------------|
| `Spawning` | No | Workspace being created, agent starting |
| `Working` | No | Agent is actively writing code |
| `PrDraft` | No | Draft pull request created |
| `PrOpen` | No | Pull request opened for review |
| `CiPassing` | No | CI checks are green |
| `CiFailed` | No | CI checks failed |
| `CiFixSent` | No | Agent sent a fix for CI |
| `CiFixFailed` | No | CI fix attempt also failed |
| `ReviewPending` | No | Awaiting code review |
| `ChangesRequested` | No | Reviewer requested changes |
| `Approved` | No | PR approved |
| `MergeConflicts` | No | Merge conflicts detected |
| `Merged` | Yes | PR merged successfully |
| `Done` | Yes | Session completed normally |
| `Exited` | No | Agent exited unexpectedly (can be restored) |
| `Killed` | Yes | Manually terminated |

## Activity States

Independent of the session status, each session has an **activity state** reflecting the agent process:

| Activity | Description |
|----------|-------------|
| `Active` | Agent is actively working |
| `Ready` | Agent is idle, waiting for input |
| `Idle` | No recent activity |
| `WaitingInput` | Agent explicitly waiting for user input |
| `Blocked` | Blocked on an external resource |
| `Exited` | Agent process has exited |

## Lifecycle Polling

The **LifecycleManager** runs a continuous polling loop that:

1. Queries the tracker plugin for PR/CI status
2. Queries the SCM plugin for review state
3. Compares external state against the current session status
4. Triggers status transitions
5. Fires configured [reactions](./reactions.md)
6. Emits events to the event bus and database

## Session Operations

| Operation | CLI Command | Effect |
|-----------|-------------|--------|
| Spawn | `ennio spawn <project>` | Creates workspace, starts agent |
| Send | `ennio send <session> <msg>` | Sends text to the running agent |
| Kill | `ennio session kill <id>` | Terminates the agent and marks session as `Killed` |
| Restore | `ennio session restore <id>` | Restarts an `Exited` session in its existing workspace |
| Info | `ennio session info <id>` | Shows session details, status history, recent events |
| List | `ennio session list [project]` | Lists all sessions, optionally filtered by project |
