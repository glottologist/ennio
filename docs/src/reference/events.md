# Event System

Ennio emits structured events for every significant state change. Events flow through multiple channels for real-time and historical access.

## Event Structure

Every event contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `EventId` | Unique identifier |
| `event_type` | `EventType` | What happened |
| `priority` | `EventPriority` | Severity level |
| `session_id` | `SessionId` | Affected session |
| `project_id` | `ProjectId` | Owning project |
| `timestamp` | `DateTime<Utc>` | When it happened |
| `message` | `String` | Human-readable description |
| `data` | `JSON` | Structured payload |

## Event Types

### Session Events

| Type | Description |
|------|-------------|
| `SessionSpawned` | New session created and agent launched |
| `SessionWorking` | Agent began active work |
| `SessionExited` | Agent process exited unexpectedly |
| `SessionKilled` | Session manually terminated |
| `SessionRestored` | Exited session restarted |
| `SessionCleaned` | Session workspace cleaned up |

### Status Events

| Type | Description |
|------|-------------|
| `StatusChanged` | Session transitioned to a new status |
| `ActivityChanged` | Session activity state changed |

### Pull Request Events

| Type | Description |
|------|-------------|
| `PrCreated` | Pull request created |
| `PrUpdated` | Pull request updated (new commits) |
| `PrMerged` | Pull request merged |
| `PrClosed` | Pull request closed without merging |

### CI Events

| Type | Description |
|------|-------------|
| `CiPassing` | CI checks are green |
| `CiFailing` | CI checks failed |
| `CiFixSent` | Agent pushed a CI fix |
| `CiFixFailed` | CI fix attempt also failed |

### Review Events

| Type | Description |
|------|-------------|
| `ReviewPending` | Awaiting code review |
| `ReviewApproved` | PR approved |
| `ReviewChangesRequested` | Reviewer requested changes |
| `ReviewCommentsSent` | Review comments forwarded to agent |

### Merge Events

| Type | Description |
|------|-------------|
| `MergeReady` | PR ready to merge (approved + CI green) |
| `MergeConflicts` | Merge conflicts detected |
| `MergeCompleted` | PR merged successfully |

### Reaction Events

| Type | Description |
|------|-------------|
| `ReactionTriggered` | A reaction rule fired |
| `ReactionEscalated` | Reaction escalated after timeout |
| `AllComplete` | All project sessions completed |

### Node Events

| Type | Description |
|------|-------------|
| `NodeConnected` | Connected to remote node |
| `NodeDisconnected` | Disconnected from remote node |
| `NodeLaunched` | Remote node daemon started |
| `NodeHealthCheck` | Node health check performed |

## Event Priority

| Priority | Use Case |
|----------|----------|
| `Info` | Status updates, completions |
| `Action` | Something needs attention |
| `Urgent` | Immediate human attention needed |
| `Critical` | System-level failures |

Priorities are ordered: `Info < Action < Urgent < Critical`.

## Event Channels

### EventBus (In-Process)

Tokio broadcast channel with capacity 1024. Subscribers receive events in real-time. Used by the lifecycle manager, web API (SSE), and internal consumers.

```rust
let rx = event_bus.subscribe(EventType::CiFailing);
while let Ok(event) = rx.recv().await {
    // handle event
}
```

### NATS (Distributed)

Events are published to category-based NATS topics. Each event type maps to a topic category:

| Category | Topic Format | Event Types |
|----------|-------------|-------------|
| Sessions | `ennio.sessions.{project_id}.{action}` | Spawned, Working, Exited, Killed, Restored, Cleaned, StatusChanged, ActivityChanged |
| Pull Requests | `ennio.pr.{project_id}.{action}` | PrCreated, PrUpdated, PrMerged, PrClosed |
| CI | `ennio.ci.{project_id}.{action}` | CiPassing, CiFailing, CiFixSent, CiFixFailed |
| Reviews | `ennio.review.{project_id}.{action}` | ReviewPending, ReviewApproved, ReviewChangesRequested, ReviewCommentsSent |
| Merge | `ennio.merge.{project_id}.{action}` | MergeReady, MergeConflicts, MergeCompleted |
| Reactions | `ennio.reactions.{project_id}.{action}` | ReactionTriggered, ReactionEscalated |
| Lifecycle | `ennio.lifecycle.{action}` | AllComplete |
| Nodes | `ennio.node.{host}.{action}` | NodeConnected, NodeDisconnected, NodeLaunched, NodeHealthCheck |
| Commands | `ennio.commands.{command}` | Shutdown and other control commands |
| Metrics | `ennio.metrics.{action}` | Metric collection events |
| Dashboard | `ennio.dashboard.{action}` | Dashboard update events |

External systems can subscribe to patterns:
- `ennio.sessions.my-project.*` — all session events for a project
- `ennio.ci.my-project.*` — all CI events for a project
- `ennio.node.build-server.*` — all events from a remote node
- `ennio.lifecycle.*` — all lifecycle events

Topic segments are validated: alphanumeric, underscore, and hyphen characters only. No spaces or dots within segments.

### SQLite (Persistent)

All events are persisted to the `events` table for history, debugging, and replay. Query via the REST API or directly from the database.
