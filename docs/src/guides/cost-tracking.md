# Cost Tracking

Ennio includes a financial ledger for tracking AI agent costs using double-entry bookkeeping.

## Overview

Every token consumed by an agent session is recorded as a `CostEntry` and posted to the ledger as a double-entry `Transfer`. This gives you accurate, auditable cost data per session, per project, and globally.

## Cost Entries

Each cost entry records:

| Field | Description |
|-------|-------------|
| `session_id` | Which session incurred the cost |
| `project_id` | Which project the session belongs to |
| `input_tokens` | Number of input tokens consumed |
| `output_tokens` | Number of output tokens generated |
| `cost_usd` | Dollar cost of the API call |
| `model` | Model used (e.g., `opus`, `sonnet`) |
| `timestamp` | When the cost was incurred |

## Querying Costs

The ledger trait provides:

```
get_session_cost(session_id) → Decimal
get_project_cost(project_id) → Decimal
get_total_cost()             → Decimal
```

All monetary values use `rust_decimal::Decimal` for precision.

## Budgets

Set spending limits at three scopes:

| Scope | Description |
|-------|-------------|
| `Global` | Total spend across all projects |
| `Project` | Spend limit for a specific project |
| `Session` | Spend limit for a single session |

Each budget has a period:

| Period | Description |
|--------|-------------|
| `Daily` | Resets every 24 hours |
| `Monthly` | Resets every calendar month |
| `Total` | Lifetime limit, never resets |

### Budget Checking

Before spawning expensive operations, check the budget:

```
check_budget(project_id, amount) → BudgetStatus
```

`BudgetStatus` tells you:
- `within_budget` — whether the amount fits
- `used` / `limit` / `remaining` — current state
- `percent_used` — utilization percentage

## Double-Entry Bookkeeping

The ledger uses proper accounting:

- **Asset accounts** — track what you own
- **Liability accounts** — track what you owe
- **Revenue accounts** — track income
- **Expense accounts** — track spending

Every cost entry creates a `Transfer` that debits an expense account and credits an asset account, maintaining the accounting equation.

## Implementation

Ennio ships with `InMemoryLedger` — a thread-safe in-memory implementation using `RwLock`. The `Ledger` trait is async and designed for future backing stores (database, TigerBeetle, etc.).
