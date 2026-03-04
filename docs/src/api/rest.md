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
GET /health
```

Returns `200 OK` with no authentication required. Use for load balancer health checks.

---

### List Sessions

```
GET /api/v1/sessions
```

Returns all active sessions.

**Response:**

```json
[
  {
    "id": "myapp-abc123",
    "project": "my-app",
    "status": "Working",
    "activity": "Active",
    "branch": "feat/auth",
    "agent": "claude-code",
    "created_at": "2026-03-04T10:00:00Z"
  }
]
```

---

### Get Session

```
GET /api/v1/sessions/{id}
```

Returns details for a specific session.

**Response:** `200` with session object, or `404` if not found.

---

### Spawn Session

```
POST /api/v1/sessions
```

**Request Body:**

```json
{
  "project": "my-app",
  "issue": "42",
  "prompt": "Add input validation",
  "branch": "feat/validation",
  "role": "implementer"
}
```

All fields except `project` are optional. Provide either `issue` or `prompt`.

**Response:** `201 Created` with the new session object.

---

### Kill Session

```
DELETE /api/v1/sessions/{id}
```

Terminates the agent and marks the session as `Killed`.

**Response:** `200 OK` or `404` if not found.

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

**Response:** `200 OK` or `404` if session not found.

## CORS

Configure allowed origins:

```yaml
cors_origins:
  - http://localhost:3000
  - https://dashboard.example.com
```

If no origins are configured, CORS headers are not sent.
