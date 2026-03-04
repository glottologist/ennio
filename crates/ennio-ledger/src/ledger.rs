use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::LedgerError;
use crate::types::{Budget, BudgetStatus, CostEntry, Transfer};

/// Trait for financial ledger operations: cost recording, budget checking, and transfer management.
#[async_trait]
pub trait Ledger: Send + Sync {
    /// Record a cost entry and create the corresponding double-entry transfer.
    async fn record_cost(&self, entry: &CostEntry) -> Result<Transfer, LedgerError>;

    async fn get_session_cost(&self, session_id: &str) -> Result<Decimal, LedgerError>;

    async fn get_project_cost(&self, project_id: &str) -> Result<Decimal, LedgerError>;

    async fn get_total_cost(&self) -> Result<Decimal, LedgerError>;

    async fn check_budget(
        &self,
        project_id: &str,
        amount: &Decimal,
    ) -> Result<BudgetStatus, LedgerError>;

    /// Create or update a budget.
    async fn set_budget(&self, budget: &Budget) -> Result<(), LedgerError>;

    async fn get_budgets(&self, project_id: Option<&str>) -> Result<Vec<Budget>, LedgerError>;
}
