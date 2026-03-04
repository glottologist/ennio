use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Classification of ledger accounts following double-entry bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Revenue,
    Expense,
}

/// A ledger account that tracks a monetary balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: AccountType,
    pub balance: Decimal,
}

impl Account {
    pub fn new(id: impl Into<String>, name: impl Into<String>, account_type: AccountType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            account_type,
            balance: Decimal::ZERO,
        }
    }
}

/// A double-entry transfer between two accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    pub debit_account_id: String,
    pub credit_account_id: String,
    pub amount: Decimal,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub session_id: Option<String>,
    pub metadata: serde_json::Value,
}

/// A record of cost incurred by an LLM invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostEntry {
    pub session_id: String,
    pub project_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Decimal,
    pub model: String,
    pub timestamp: DateTime<Utc>,
}

/// Time period over which a budget is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetPeriod {
    Daily,
    Monthly,
    Total,
}

/// Scope to which a budget applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetScope {
    Global,
    Project,
    Session,
}

/// A spending limit configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub id: String,
    pub project_id: Option<String>,
    pub scope: BudgetScope,
    pub period: BudgetPeriod,
    pub limit_usd: Decimal,
    pub used_usd: Decimal,
}

impl Budget {
    pub fn remaining(&self) -> Decimal {
        let diff = self.limit_usd - self.used_usd;
        if diff < Decimal::ZERO {
            Decimal::ZERO
        } else {
            diff
        }
    }

    pub fn is_within_budget(&self, additional: &Decimal) -> bool {
        self.used_usd + additional <= self.limit_usd
    }
}

/// Result of checking whether a proposed spend fits within budget constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub within_budget: bool,
    pub used: Decimal,
    pub limit: Decimal,
    pub remaining: Decimal,
    pub percent_used: f64,
}

impl BudgetStatus {
    /// Build a status from a budget and a proposed additional amount.
    pub fn from_budget(budget: &Budget, additional: &Decimal) -> Self {
        let new_used = budget.used_usd + additional;
        let within_budget = new_used <= budget.limit_usd;
        let diff = budget.limit_usd - new_used;
        let remaining = if diff < Decimal::ZERO {
            Decimal::ZERO
        } else {
            diff
        };

        let percent_used = if budget.limit_usd.is_zero() {
            if new_used.is_zero() { 0.0 } else { 100.0 }
        } else {
            use rust_decimal::prelude::ToPrimitive;
            let ratio = new_used / budget.limit_usd;
            ratio.to_f64().unwrap_or(0.0) * 100.0
        };

        Self {
            within_budget,
            used: new_used,
            limit: budget.limit_usd,
            remaining,
            percent_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rust_decimal::prelude::FromPrimitive;

    use super::*;

    fn arb_decimal(min: f64, max: f64) -> impl Strategy<Value = Decimal> {
        (min..max).prop_map(|v| Decimal::from_f64(v).unwrap_or(Decimal::ZERO))
    }

    proptest! {
        #[test]
        fn budget_within_when_used_plus_amount_lte_limit(
            limit in arb_decimal(1.0, 10000.0),
        ) {
            let budget = Budget {
                id: "b1".to_string(),
                project_id: None,
                scope: BudgetScope::Global,
                period: BudgetPeriod::Total,
                limit_usd: limit,
                used_usd: Decimal::ZERO,
            };
            let status = BudgetStatus::from_budget(&budget, &Decimal::ZERO);
            prop_assert!(status.within_budget);
            prop_assert_eq!(status.remaining, limit);
        }

        #[test]
        fn budget_exceeded_when_over_limit(
            limit in arb_decimal(1.0, 5000.0),
            overage in arb_decimal(0.01, 1000.0),
        ) {
            let used = limit + overage;
            let budget = Budget {
                id: "b1".to_string(),
                project_id: None,
                scope: BudgetScope::Global,
                period: BudgetPeriod::Total,
                limit_usd: limit,
                used_usd: used,
            };
            let status = BudgetStatus::from_budget(&budget, &Decimal::ZERO);
            prop_assert!(!status.within_budget);
        }

        #[test]
        fn remaining_never_negative(
            limit in arb_decimal(0.0, 10000.0),
            used in arb_decimal(0.0, 20000.0),
        ) {
            let budget = Budget {
                id: "b1".to_string(),
                project_id: None,
                scope: BudgetScope::Global,
                period: BudgetPeriod::Total,
                limit_usd: limit,
                used_usd: used,
            };
            prop_assert!(budget.remaining() >= Decimal::ZERO);
        }

        #[test]
        fn is_within_budget_consistent_with_status(
            limit in arb_decimal(1.0, 10000.0),
            used in arb_decimal(0.0, 5000.0),
            additional in arb_decimal(0.0, 5000.0),
        ) {
            let budget = Budget {
                id: "b1".to_string(),
                project_id: None,
                scope: BudgetScope::Global,
                period: BudgetPeriod::Total,
                limit_usd: limit,
                used_usd: used,
            };
            let status = BudgetStatus::from_budget(&budget, &additional);
            prop_assert_eq!(budget.is_within_budget(&additional), status.within_budget);
        }

        #[test]
        fn percent_used_bounded(
            limit in arb_decimal(0.01, 10000.0),
            used in arb_decimal(0.0, 10000.0),
        ) {
            let budget = Budget {
                id: "b1".to_string(),
                project_id: None,
                scope: BudgetScope::Global,
                period: BudgetPeriod::Total,
                limit_usd: limit,
                used_usd: Decimal::ZERO,
            };
            let status = BudgetStatus::from_budget(&budget, &used);
            prop_assert!(status.percent_used >= 0.0);
        }
    }

    #[test]
    fn budget_status_zero_limit_zero_used() {
        let budget = Budget {
            id: "b1".to_string(),
            project_id: None,
            scope: BudgetScope::Global,
            period: BudgetPeriod::Total,
            limit_usd: Decimal::ZERO,
            used_usd: Decimal::ZERO,
        };
        let status = BudgetStatus::from_budget(&budget, &Decimal::ZERO);
        assert!(status.within_budget);
        assert_eq!(status.percent_used, 0.0);
    }

    #[test]
    fn budget_status_zero_limit_nonzero_amount() {
        let budget = Budget {
            id: "b1".to_string(),
            project_id: None,
            scope: BudgetScope::Global,
            period: BudgetPeriod::Total,
            limit_usd: Decimal::ZERO,
            used_usd: Decimal::ZERO,
        };
        let amount = Decimal::from_f64(1.0).unwrap();
        let status = BudgetStatus::from_budget(&budget, &amount);
        assert!(!status.within_budget);
        assert_eq!(status.percent_used, 100.0);
    }
}
