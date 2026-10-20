use chrono::{Datelike, Months, NaiveDate};
use sqlx::{AssertSqlSafe, Row};

use crate::api::*;
use crate::{Caller, Error, Result, Store, tags as norm_tags, visible};

/// The first day of the month `YYYY-MM`.
pub(crate) fn month_start(m: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d").map_err(|_| Error::bad(format!("invalid month '{m}', expected YYYY-MM")))
}

impl Store {
    /// Every budget the caller set, with what the month (this one unless given) has used of each.
    pub async fn budgets(&self, c: &Caller, q: BudgetQuery) -> Result<Vec<Budget>> {
        c.need("read")?;
        let today = self.today();
        let start = match q.month.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
            Some(m) => month_start(m)?,
            None => today.with_day(1).unwrap(),
        };
        let next = start + Months::new(1);
        let prev = start - Months::new(1);
        let rows = sqlx::query("SELECT id, tag, amount FROM budgets WHERE user_id = $1 ORDER BY tag").bind(c.user_id).fetch_all(&self.pool).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let tags: Vec<String> = rows.iter().map(|r| r.get("tag")).collect();
        // one pass over both months: what each tag took, as money out (positive) less money back
        let sql = format!(
            "SELECT g.tag, \
               COALESCE(SUM(CASE WHEN t.date >= $2 THEN -t.amount END), 0)::BIGINT AS now, \
               COALESCE(SUM(CASE WHEN t.date < $2 THEN -t.amount END), 0)::BIGINT AS before \
             FROM tx_tags g JOIN transactions t ON t.id = g.tx_id JOIN accounts a ON a.id = t.account_id \
             WHERE {} AND t.kind <> 'transfer' AND g.tag = ANY($1) AND t.date >= $3 AND t.date < $4 GROUP BY g.tag",
            visible(c.user_id)
        );
        let sums = sqlx::query(AssertSqlSafe(sql))
            .bind(&tags)
            .bind(start.to_string())
            .bind(prev.to_string())
            .bind(next.to_string())
            .fetch_all(&self.pool)
            .await?;
        let month = format!("{:04}-{:02}", start.year(), start.month());
        Ok(rows
            .iter()
            .map(|r| {
                let tag: String = r.get("tag");
                let s = sums.iter().find(|s| s.get::<String, _>("tag") == tag);
                Budget {
                    id: r.get("id"),
                    amount: r.get("amount"),
                    month: month.clone(),
                    spent: s.map(|s| s.get::<i64, _>("now")).unwrap_or(0),
                    last_month: s.map(|s| s.get::<i64, _>("before")).unwrap_or(0),
                    tag,
                }
            })
            .collect())
    }

    /// Set the monthly limit on a tag. A tag has one budget: setting it again changes the amount.
    pub async fn set_budget(&self, c: &Caller, b: SetBudget) -> Result<Budget> {
        c.need("add")?;
        let tag = norm_tags(&[b.tag]).into_iter().next().ok_or_else(|| Error::bad("say which tag the budget is for"))?;
        if b.amount <= 0 {
            return Err(Error::bad("a budget must be above zero"));
        }
        let id: i64 = sqlx::query("INSERT INTO budgets (user_id, tag, amount) VALUES ($1, $2, $3) ON CONFLICT (user_id, tag) DO UPDATE SET amount = EXCLUDED.amount RETURNING id")
            .bind(c.user_id)
            .bind(&tag)
            .bind(b.amount)
            .fetch_one(&self.pool)
            .await?
            .get(0);
        self.budgets(c, BudgetQuery::default()).await?.into_iter().find(|x| x.id == id).ok_or(Error::NotFound("budget"))
    }

    pub async fn delete_budget(&self, c: &Caller, id: i64) -> Result<()> {
        c.need("edit")?;
        let n = sqlx::query("DELETE FROM budgets WHERE id = $1 AND user_id = $2").bind(id).bind(c.user_id).execute(&self.pool).await?.rows_affected();
        if n == 0 { Err(Error::NotFound("budget")) } else { Ok(()) }
    }
}
