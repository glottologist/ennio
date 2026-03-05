# REST API

Ennio exposes a REST API on the configured `port` (default: 3000).

## Authentication

All endpoints under `/api/v1` (except `/health`) require a bearer token when `api_token` is set in the configuration:

```
Authorization: Bearer <token>
```

The token is compared using constant-time SHA-256 hashing.

## Endpoints

### Health Check

```
GET /api/v1/health
```

Returns `200 OK` with no authentication required. Use for load balancer health checks.

**Response:**

```json
{
  "data": "ok"
}
```

---

### List Sessions

```
GET /api/v1/sessions[?project_id=<id>]
```

Returns all active sessions, optionally filtered by project.

| Parameter | Location | Required | Description |
|-----------|----------|----------|-------------|
| `project_id` | query | No | Filter sessions by project ID |

**Response:**

```json
{
  "data": [
    {
      "id": "myapp-abc123",
      "project_id": "my-app",
      "status": "Working",
      "activity": "Active",
      "branch": "feat/auth",
      "pr_url": null,
      "agent_name": "claude-code"
    }
  ]
}
```

---

### Get Session

```
GET /api/v1/sessions/{id}
```

Returns details for a specific session.

**Response:** `200` with session object wrapped in `data`, or `404` if not found.

```json
{
  "data": {
    "id": "myapp-abc123",
    "project_id": "my-app",
    "status": "CiPassing",
    "activity": "Idle",
    "branch": "feat/auth",
    "pr_url": "https://github.com/org/repo/pull/42",
    "agent_name": "claude-code"
  }
}
```

---

### Spawn Session

```
POST /api/v1/sessions
```

**Request Body:**

```json
{
  "project_id": "my-app",
  "issue_id": "42",
  "prompt": "Add input validation",
  "branch": "feat/validation",
  "role": "implementer"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `project_id` | `String` | Yes | Project to spawn in |
| `issue_id` | `String` | No | Issue ID to work on (fetched from tracker) |
| `prompt` | `String` | No | Direct prompt for the agent |
| `branch` | `String` | No | Git branch name to use |
| `role` | `String` | No | Session role |

Provide either `issue_id` or `prompt` (or both).

**Response:** `200` with the new session object wrapped in `data`.

---

### Kill Session

```
DELETE /api/v1/sessions/{id}
```

Terminates the agent and marks the session as `Killed`.

**Response:**

```json
{
  "data": "killed"
}
```

Returns `404` if session not found.

---

### Send Message to Session

```
POST /api/v1/sessions/{id}/send
```

**Request Body:**

```json
{
  "message": "Also add unit tests for the validation logic"
}
```

Sends text input to the running agent via the runtime plugin.

**Response:**

```json
{
  "data": "sent"
}
```

Returns `404` if session not found.

## Error Responses

All errors return a JSON body:

```json
{
  "error": "Session not found",
  "code": 404
}
```

### HTTP Status Code Mapping

| Status | Trigger |
|--------|---------|
| `200` | Success |
| `400` | Invalid ID, configuration errors |
| `402` | Budget exceeded |
| `404` | Entity not found |
| `409` | Entity already exists |
| `504` | Operation timed out |
| `500` | Internal server error |

## CORS

Configure allowed origins:

```yaml
cors_origins:
  - http://localhost:3000
  - https://dashboard.example.com
```

Allowed methods: `GET`, `POST`, `DELETE`. If no origins are configured, CORS headers are not sent.
