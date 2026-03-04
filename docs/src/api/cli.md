# CLI Reference

## Global Usage

```
ennio [COMMAND]
```

## Commands

### `init`

Initialize a new Ennio configuration file.

```bash
ennio init <path>
```

Creates `ennio.yaml` at the specified path with default values. Refuses to overwrite existing files.

---

### `start`

Start the orchestrator lifecycle loop.

```bash
ennio start
```

Boots the full orchestrator: SQLite database, NATS connection, plugin registry, web API server, and lifecycle polling loop. Runs until interrupted (`Ctrl+C`) or stopped via `ennio stop`.

---

### `stop`

Stop a running orchestrator.

```bash
ennio stop
```

Sends a shutdown signal via NATS.

---

### `status`

Show status of all sessions.

```bash
ennio status [project]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `project` | No | Filter sessions by project name |

---

### `spawn`

Spawn a new agent session.

```bash
ennio spawn <project> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `--issue` | `-i` | Issue ID to work on (fetched from tracker) |
| `--prompt` | `-p` | Direct prompt for the agent |
| `--branch` | `-b` | Git branch name to use |
| `--role` | `-r` | Session role |

Provide either `--issue` or `--prompt` (or both).

---

### `send`

Send a message to a running session.

```bash
ennio send <session-id> <message>
```

The message is delivered to the agent via the runtime plugin (e.g., tmux send-keys).

---

### `session`

Manage individual sessions.

#### `session info`

```bash
ennio session info <id>
```

Displays session details including status, activity, branch, PR URL, and recent events.

#### `session kill`

```bash
ennio session kill <id>
```

Terminates the agent and marks the session as `Killed`.

#### `session restore`

```bash
ennio session restore <id>
```

Restarts an `Exited` session in its existing workspace.

#### `session list`

```bash
ennio session list [project]
```

Lists all sessions, optionally filtered by project.

---

### `dashboard`

Open the web dashboard.

```bash
ennio dashboard [--port <port>]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--port` | 3000 | Port for the dashboard web server |

---

### `open`

Open a session's terminal.

```bash
ennio open <session-id>
```

Prints the `tmux attach` command to connect to the agent's terminal session.

---

### `node`

Manage remote node daemons.

#### `node status`

```bash
ennio node status [host]
```

Check health of a remote `ennio-node` daemon.

#### `node list`

```bash
ennio node list
```

List all projects configured with remote node connections.

#### `node connect`

```bash
ennio node connect <project>
```

Establish an SSH tunnel and connect to the remote node for the specified project.

#### `node disconnect`

```bash
ennio node disconnect <project>
```

Disconnect and shut down the remote node.
