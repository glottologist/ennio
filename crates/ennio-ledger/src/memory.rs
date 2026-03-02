use async_trait::async_trait;
use chrono::Utc;
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use crate::error::LedgerError;
use crate::ledger::Ledger;
use crate::types::{Budget, BudgetPeriod, BudgetScope, BudgetStatus, CostEntry, Transfer};

struct LedgerState {
    cost_entries: Vec<CostEntry>,
    transfers: Vec<Transfer>,
    budgets: Vec<Budget>,
    next_transfer_id: u64,
}

/// Thread-safe in-memory ledger suitable for testing and single-machine use.
pub struct InMemoryLedger {
    state: RwLock<LedgerState>,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(LedgerState {
                cost_entries: Vec::new(),
                transfers: Vec::new(),
                budgets: Vec::new(),
                next_transfer_id: 1,
            }),
        }
    }
}

impl Default for InMemoryLedger {
    fn default() -> Self {
        Self::new()
    }
}

fn find_matching_budget<'a>(budgets: &'a [Budget], project_id: &str) -> Option<&'a Budget> {
    budgets
        .iter()
        .find(|b| b.scope == BudgetScope::Project && b.project_id.as_deref() == Some(project_id))
        .or_else(|| budgets.iter().find(|b| b.scope == BudgetScope::Global))
}

fn build_default_budget_status(amount: &Decimal) -> BudgetStatus {
    let default_budget = Budget {
        id: String::new(),
        project_id: None,
        scope: BudgetScope::Global,
        period: BudgetPeriod::Total,
        limit_usd: Decimal::MAX,
        used_usd: Decimal::ZERO,
    };
    BudgetStatus::from_budget(&default_budget, amount)
}

#[async_trait]
impl Ledger for InMemoryLedger {
    async fn record_cost(&self, entry: &CostEntry) -> Result<Transfer, LedgerError> {
        if entry.cost_usd < Decimal::ZERO {
            return Err(LedgerError::InvalidAmount {
                reason: "cost cannot be negative".to_string(),
            });
        }

        let mut state = self.state.write().await;

        let transfer_id = state.next_transfer_id;
        state.next_transfer_id =
            transfer_id
                .checked_add(1)
                .ok_or_else(|| LedgerError::Internal {
                    message: "transfer ID overflow".to_string(),
                })?;

        let transfer = Transfer {
            id: transfer_id.to_string(),
            debit_account_id: "expense:llm".to_string(),
            credit_account_id: "liability:accrued".to_string(),
            amount: entry.cost_usd,
            timestamp: Utc::now(),
            description: format!(
                "LLM cost: {} tokens in, {} tokens out, model {}",
                entry.input_tokens, entry.output_tokens, entry.model
            ),
            session_id: Some(entry.session_id.clone()), // clone: owned field from borrowed entry
            metadata: serde_json::json!({
                "project_id": entry.project_id,
                "input_tokens": entry.input_tokens,
                "output_tokens": entry.output_tokens,
                "model": entry.model,
            }),
        };

        for budget in &mut state.budgets {
            let matches = match budget.scope {
                BudgetScope::Global => true,
                BudgetScope::Project => budget.project_id.as_deref() == Some(&entry.project_id),
                BudgetScope::Session => false,
            };
            if matches {
                budget.used_usd += entry.cost_usd;
            }
        }

        state.cost_entries.push(entry.clone()); // clone: storing borrowed entry into owned Vec
        state.transfers.push(transfer.clone()); // clone: returning owned Transfer while also storing
        Ok(transfer)
    }

    async fn get_session_cost(&self, session_id: &str) -> Result<Decimal, LedgerError> {
        let state = self.state.read().await;
        let total = state
            .cost_entries
            .iter()
            .filter(|e| e.session_id == session_id)
            .map(|e| e.cost_usd)
            .sum();
        Ok(total)
    }

    async fn get_project_cost(&self, project_id: &str) -> Result<Decimal, LedgerError> {
        let state = self.state.read().await;
        let total = state
            .cost_entries
            .iter()
            .filter(|e| e.project_id == project_id)
            .map(|e| e.cost_usd)
            .sum();
        Ok(total)
    }

    async fn get_total_cost(&self) -> Result<Decimal, LedgerError> {
        let state = self.state.read().await;
        let total = state.cost_entries.iter().map(|e| e.cost_usd).sum();
        Ok(total)
    }

    async fn check_budget(
        &self,
        project_id: &str,
        amount: &Decimal,
    ) -> Result<BudgetStatus, LedgerError> {
        let state = self.state.read().await;
        match find_matching_budget(&state.budgets, project_id) {
            Some(budget) => Ok(BudgetStatus::from_budget(budget, amount)),
            None => Ok(build_default_budget_status(amount)),
        }
    }

    async fn set_budget(&self, budget: &Budget) -> Result<(), LedgerError> {
        if budget.limit_usd < Decimal::ZERO {
            return Err(LedgerError::InvalidAmount {
                reason: "budget limit cannot be negative".to_string(),
            });
        }

        let mut state = self.state.write().await;

        if let Some(existing) = state.budgets.iter_mut().find(|b| b.id == budget.id) {
            existing.project_id = budget.project_id.clone(); // clone: Option<String> from borrowed to owned
            existing.scope = budget.scope;
            existing.period = budget.period;
            existing.limit_usd = budget.limit_usd;
            existing.used_usd = budget.used_usd;
        } else {
            state.budgets.push(budget.clone()); // clone: storing borrowed budget into owned Vec
        }

        Ok(())
    }

    async fn get_budgets(&self, project_id: Option<&str>) -> Result<Vec<Budget>, LedgerError> {
        let state = self.state.read().await;
        let results: Vec<Budget> = match project_id {
            Some(pid) => state
                .budgets
                .iter()
                .filter(|b| b.project_id.as_deref() == Some(pid) || b.scope == BudgetScope::Global)
                .cloned() // clone: returning owned copies from borrowed RwLock guard
                .collect(),
            None => state.budgets.clone(), // clone: returning owned copies from borrowed RwLock guard
        };
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use proptest::prelude::*;
    use rust_decimal::prelude::FromPrimitive;

    use super::*;

    fn make_cost_entry(session_id: &str, project_id: &str, cost: Decimal) -> CostEntry {
        CostEntry {
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: cost,
            model: "test-model".to_string(),
            timestamp: Utc::now(),
        }
    }

    fn make_budget(id: &str, project_id: Option<&str>, limit: Decimal) -> Budget {
        Budget {
            id: id.to_string(),
            project_id: project_id.map(String::from),
            scope: if project_id.is_some() {
                BudgetScope::Project
            } else {
                BudgetScope::Global
            },
            period: BudgetPeriod::Total,
            limit_usd: limit,
            used_usd: Decimal::ZERO,
        }
    }

    fn arb_positive_decimal() -> impl Strategy<Value = Decimal> {
        (1u64..100_000u64).prop_map(|v| Decimal::from_u64(v).unwrap_or(Decimal::ONE))
    }

    proptest! {
        #[test]
        fn record_cost_returns_matching_transfer(
            cost_cents in 1u64..100_000u64,
        ) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let ledger = InMemoryLedger::new();
                let cost = Decimal::from_u64(cost_cents).unwrap();
                let entry = make_cost_entry("s1", "p1", cost);
                let transfer = ledger.record_cost(&entry).await.unwrap();
                prop_assert_eq!(transfer.amount, cost);
                prop_assert!(transfer.session_id.as_deref() == Some("s1"));
                Ok(())
            })?;
        }

        #[test]
        fn session_cost_accumulates(
            costs in proptest::collection::vec(1u64..10_000u64, 1..10),
        ) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let ledger = InMemoryLedger::new();
                let mut expected = Decimal::ZERO;
                for c in &costs {
                    let cost = Decimal::from_u64(*c).unwrap();
                    expected += cost;
                    let entry = make_cost_entry("s1", "p1", cost);
                    ledger.record_cost(&entry).await.unwrap();
                }
                let actual = ledger.get_session_cost("s1").await.unwrap();
                prop_assert_eq!(actual, expected);
                Ok(())
            })?;
        }

        #[test]
        fn budget_check_within_when_under_limit(
            limit in arb_positive_decimal(),
        ) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let ledger = InMemoryLedger::new();
                let budget = make_budget("b1", Some("p1"), limit);
                ledger.set_budget(&budget).await.unwrap();
                let status = ledger.check_budget("p1", &Decimal::ZERO).await.unwrap();
                prop_assert!(status.within_budget);
                Ok(())
            })?;
        }

        #[test]
        fn budget_tracks_usage_after_costs(
            limit_cents in 10_000u64..100_000u64,
            cost_cents in 1u64..5_000u64,
        ) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let ledger = InMemoryLedger::new();
                let limit = Decimal::from_u64(limit_cents).unwrap();
                let cost = Decimal::from_u64(cost_cents).unwrap();
                let budget = make_budget("b1", Some("p1"), limit);
                ledger.set_budget(&budget).await.unwrap();
                let entry = make_cost_entry("s1", "p1", cost);
                ledger.record_cost(&entry).await.unwrap();
                let status = ledger.check_budget("p1", &Decimal::ZERO).await.unwrap();
                prop_assert_eq!(status.used, cost);
                prop_assert_eq!(status.remaining, limit - cost);
                Ok(())
            })?;
        }
    }

    #[tokio::test]
    async fn record_cost_negative_rejected() {
        let ledger = InMemoryLedger::new();
        let entry = make_cost_entry("s1", "p1", Decimal::from_f64(-1.0).unwrap());
        let result = ledger.record_cost(&entry).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_project_cost_sums_correctly() {
        let ledger = InMemoryLedger::new();
        let e1 = make_cost_entry("s1", "p1", Decimal::from_f64(10.0).unwrap());
        let e2 = make_cost_entry("s2", "p1", Decimal::from_f64(20.0).unwrap());
        let e3 = make_cost_entry("s3", "p2", Decimal::from_f64(5.0).unwrap());
        ledger.record_cost(&e1).await.unwrap();
        ledger.record_cost(&e2).await.unwrap();
        ledger.record_cost(&e3).await.unwrap();

        let p1_cost = ledger.get_project_cost("p1").await.unwrap();
        let p2_cost = ledger.get_project_cost("p2").await.unwrap();
        let total = ledger.get_total_cost().await.unwrap();

        assert_eq!(p1_cost, Decimal::from_f64(30.0).unwrap());
        assert_eq!(p2_cost, Decimal::from_f64(5.0).unwrap());
        assert_eq!(total, Decimal::from_f64(35.0).unwrap());
    }

    #[tokio::test]
    async fn session_isolation() {
        let ledger = InMemoryLedger::new();
        let e1 = make_cost_entry("s1", "p1", Decimal::from_f64(10.0).unwrap());
        let e2 = make_cost_entry("s2", "p1", Decimal::from_f64(20.0).unwrap());
        ledger.record_cost(&e1).await.unwrap();
        ledger.record_cost(&e2).await.unwrap();

        assert_eq!(
            ledger.get_session_cost("s1").await.unwrap(),
            Decimal::from_f64(10.0).unwrap()
        );
        assert_eq!(
            ledger.get_session_cost("s2").await.unwrap(),
            Decimal::from_f64(20.0).unwrap()
        );
    }

    #[tokio::test]
    async fn set_budget_updates_existing() {
        let ledger = InMemoryLedger::new();
        let b1 = make_budget("b1", Some("p1"), Decimal::from_f64(100.0).unwrap());
        ledger.set_budget(&b1).await.unwrap();

        let mut b1_updated = b1;
        b1_updated.limit_usd = Decimal::from_f64(200.0).unwrap();
        ledger.set_budget(&b1_updated).await.unwrap();

        let budgets = ledger.get_budgets(Some("p1")).await.unwrap();
        assert_eq!(budgets.len(), 1);
        assert_eq!(budgets[0].limit_usd, Decimal::from_f64(200.0).unwrap());
    }

    #[tokio::test]
    async fn get_budgets_filters_by_project() {
        let ledger = InMemoryLedger::new();
        let global = make_budget("g1", None, Decimal::from_f64(1000.0).unwrap());
        let p1_budget = make_budget("b1", Some("p1"), Decimal::from_f64(100.0).unwrap());
        let p2_budget = make_budget("b2", Some("p2"), Decimal::from_f64(200.0).unwrap());
        ledger.set_budget(&global).await.unwrap();
        ledger.set_budget(&p1_budget).await.unwrap();
        ledger.set_budget(&p2_budget).await.unwrap();

        let all = ledger.get_budgets(None).await.unwrap();
        assert_eq!(all.len(), 3);

        let p1_results = ledger.get_budgets(Some("p1")).await.unwrap();
        assert_eq!(p1_results.len(), 2);
        assert!(p1_results.iter().any(|b| b.id == "g1"));
        assert!(p1_results.iter().any(|b| b.id == "b1"));
    }

    #[tokio::test]
    async fn check_budget_no_budget_set_allows() {
        let ledger = InMemoryLedger::new();
        let status = ledger
            .check_budget("p1", &Decimal::from_f64(100.0).unwrap())
            .await
            .unwrap();
        assert!(status.within_budget);
    }

    #[tokio::test]
    async fn set_budget_negative_limit_rejected() {
        let ledger = InMemoryLedger::new();
        let budget = Budget {
            id: "b1".to_string(),
            project_id: None,
            scope: BudgetScope::Global,
            period: BudgetPeriod::Total,
            limit_usd: Decimal::from_f64(-10.0).unwrap(),
            used_usd: Decimal::ZERO,
        };
        let result = ledger.set_budget(&budget).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn transfer_ids_are_sequential() {
        let ledger = InMemoryLedger::new();
        let e1 = make_cost_entry("s1", "p1", Decimal::from_f64(1.0).unwrap());
        let e2 = make_cost_entry("s1", "p1", Decimal::from_f64(2.0).unwrap());
        let t1 = ledger.record_cost(&e1).await.unwrap();
        let t2 = ledger.record_cost(&e2).await.unwrap();
        assert_eq!(t1.id, "1");
        assert_eq!(t2.id, "2");
    }

    #[tokio::test]
    async fn global_budget_applies_to_all_projects() {
        let ledger = InMemoryLedger::new();
        let global = Budget {
            id: "g1".to_string(),
            project_id: None,
            scope: BudgetScope::Global,
            period: BudgetPeriod::Total,
            limit_usd: Decimal::from_f64(50.0).unwrap(),
            used_usd: Decimal::ZERO,
        };
        ledger.set_budget(&global).await.unwrap();

        let e1 = make_cost_entry("s1", "p1", Decimal::from_f64(20.0).unwrap());
        let e2 = make_cost_entry("s2", "p2", Decimal::from_f64(20.0).unwrap());
        ledger.record_cost(&e1).await.unwrap();
        ledger.record_cost(&e2).await.unwrap();

        let status = ledger
            .check_budget("p1", &Decimal::from_f64(15.0).unwrap())
            .await
            .unwrap();
        assert!(!status.within_budget);
        assert_eq!(status.used, Decimal::from_f64(55.0).unwrap());
    }
}
