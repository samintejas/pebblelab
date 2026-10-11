//! Signed-in browsers, and handing a browser from the account to an app.
//!
//! Each app lives at its own address with its own storage, so a session cannot simply be shared. Instead the
//! account (where you sign in) gives the app a one-time code in the address it sends the browser to; the app
//! swaps the code for a session of its own (`redeem_login_code`). That session holds the app's scopes only and
//! hangs off the account session that made it, so signing out of either signs out of both.

use sqlx::Row;

use crate::api::*;
use crate::social::{hash, random};
use crate::{Caller, Error, Result, Store};

impl Store {
    /// A one-time code that signs this browser in to `app`. Only an account session can hand one out, and
    /// only for an app (the account needs none). Works once, for two minutes.
    pub async fn handoff(&self, c: &Caller, app: &str) -> Result<String> {
        c.need_account()?;
        let Some(info) = crate::api::app(app) else {
            return Err(Error::NotFound("app"));
        };
        if info.id == ACCOUNT.id {
            return Err(Error::bad("you are already signed in to the account"));
        }
        let parent = c.session().ok_or_else(|| Error::bad("only a signed-in browser can be handed to an app"))?;
        let code = random("lgc_", 32);
        sqlx::query("DELETE FROM login_codes WHERE created_at < utc_text(now() - interval '2 minutes')").execute(&self.pool).await?;
        sqlx::query("INSERT INTO login_codes (code_hash, user_id, app, parent_id) VALUES ($1, $2, $3, $4)")
            .bind(hash(&code))
            .bind(c.user_id)
            .bind(info.id)
            .bind(parent)
            .execute(&self.pool)
            .await?;
        Ok(code)
    }

    /// Where you are signed in: the account and each app, in each browser. Newest first.
    pub async fn sessions(&self, c: &Caller) -> Result<Vec<SessionInfo>> {
        c.need_account()?;
        let rows = sqlx::query(
            "SELECT id, app, created_at, last_used_at FROM tokens WHERE user_id = $1 AND kind = 'session' \
             AND created_at > utc_text(now() - interval '180 days') \
             AND COALESCE(last_used_at, created_at) > utc_text(now() - interval '30 days') ORDER BY id DESC",
        )
        .bind(c.user_id)
        .fetch_all(&self.pool)
        .await?;
        let me = c.session();
        Ok(rows
            .iter()
            .map(|r| {
                let id: i64 = r.get("id");
                SessionInfo { id, app: r.get("app"), created_at: r.get("created_at"), last_used_at: r.get("last_used_at"), current: Some(id) == me }
            })
            .collect())
    }

    /// End one session, and the app sessions it opened.
    pub async fn revoke_session(&self, c: &Caller, id: i64) -> Result<()> {
        c.need_account()?;
        let n = sqlx::query("DELETE FROM tokens WHERE id = $1 AND user_id = $2 AND kind = 'session'")
            .bind(id)
            .bind(c.user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 { Err(Error::NotFound("session")) } else { Ok(()) }
    }
}
