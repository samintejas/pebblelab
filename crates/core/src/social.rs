//! Signing in with another provider (Google, GitHub). The server talks to the provider; the rules for what
//! that sign-in means for an account live here.

use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::api::*;
use crate::auth::{initials_of, UNUSABLE_PASSWORD};
use crate::{Caller, Error, Result, Store, clean};

/// Providers a person can sign in with.
pub const PROVIDERS: [&str; 2] = ["google", "github"];

/// Who a provider says someone is, after it has been asked.
#[derive(Debug, Clone)]
pub struct ExternalIdentity {
    pub provider: String,
    /// The provider's own id for the person.
    pub subject: String,
    pub email: String,
    /// The provider vouches that the person controls `email`.
    pub email_verified: bool,
    pub name: String,
    /// Their profile picture as a `data:image/…` url, when the provider has one and it could be fetched.
    pub picture: Option<String>,
}

pub(crate) fn random(prefix: &str, bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    rand::fill(&mut raw[..]);
    format!("{prefix}{}", hex::encode(raw))
}

pub(crate) fn hash(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

impl Store {
    /// Start a provider sign-in: remember a one-time `state` and the PKCE `verifier` that goes with it.
    /// Returns `(state, verifier)`.
    pub async fn oauth_start(&self, provider: &str) -> Result<(String, String)> {
        self.oauth_begin(provider, None).await
    }

    /// Start a provider sign-in that adds a way to sign in to `user_id`'s account instead of signing anyone in.
    pub async fn oauth_start_link(&self, c: &Caller, provider: &str) -> Result<(String, String)> {
        if !c.is_interactive() {
            return Err(Error::Forbidden("only a signed-in person can add a way to sign in".into()));
        }
        self.oauth_begin(provider, Some(c.user_id)).await
    }

    async fn oauth_begin(&self, provider: &str, link_user: Option<i64>) -> Result<(String, String)> {
        if !PROVIDERS.contains(&provider) {
            return Err(Error::NotFound("provider"));
        }
        sqlx::query("DELETE FROM oauth_flows WHERE created_at < utc_text(now() - interval '10 minutes')").execute(&self.pool).await?;
        let (state, verifier) = (random("", 24), random("", 32));
        sqlx::query("INSERT INTO oauth_flows (state, provider, verifier, link_user) VALUES ($1, $2, $3, $4)")
            .bind(&state)
            .bind(provider)
            .bind(&verifier)
            .bind(link_user)
            .execute(&self.pool)
            .await?;
        Ok((state, verifier))
    }

    /// Whose account a provider sign-in that is adding a way to sign in belongs to. Ask before `oauth_finish`.
    pub async fn oauth_link_user(&self, state: &str) -> Result<Option<i64>> {
        Ok(sqlx::query_scalar::<_, Option<i64>>("SELECT link_user FROM oauth_flows WHERE state = $1").bind(state).fetch_optional(&self.pool).await?.flatten())
    }

    /// The provider came back: use up the `state` and give back its PKCE verifier. Works once, within ten
    /// minutes, for the provider it was made for.
    pub async fn oauth_finish(&self, provider: &str, state: &str) -> Result<String> {
        sqlx::query_scalar::<_, String>(
            "DELETE FROM oauth_flows WHERE state = $1 AND provider = $2 AND created_at > utc_text(now() - interval '10 minutes') RETURNING verifier",
        )
        .bind(state)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| Error::bad("this sign-in expired or was already used: start again"))
    }

    /// Decide whose account a provider sign-in is, and hand back a one-time code for it.
    ///
    /// - Known (provider, id): that account.
    /// - Otherwise the provider must vouch for the email. If an account has it, the two are linked; that
    ///   proves the person owns the address, which a password sign-up never did, so anything set up under
    ///   that address before is cut off: its sessions end and its password stops working.
    /// - Otherwise a new account is made, if sign-ups are open.
    pub async fn external_sign_in(&self, who: ExternalIdentity) -> Result<String> {
        if !PROVIDERS.contains(&who.provider.as_str()) {
            return Err(Error::NotFound("provider"));
        }
        let mut db = self.pool.begin().await?;
        let known: Option<i64> = sqlx::query_scalar("SELECT user_id FROM identities WHERE provider = $1 AND subject = $2")
            .bind(&who.provider)
            .bind(&who.subject)
            .fetch_optional(&mut *db)
            .await?;
        let user_id = match known {
            Some(id) => id,
            None => {
                if !who.email_verified {
                    return Err(Error::Forbidden(format!("{} has not verified your email address: verify it there and try again", who.provider)));
                }
                let email = crate::auth::check_email(&who.email)?;
                let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE lower(email) = $1 FOR UPDATE").bind(&email).fetch_optional(&mut *db).await?;
                let id = match existing {
                    Some(id) => {
                        sqlx::query("DELETE FROM tokens WHERE user_id = $1 AND kind = 'session'").bind(id).execute(&mut *db).await?;
                        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2 AND password_hash <> $1").bind(UNUSABLE_PASSWORD).bind(id).execute(&mut *db).await?;
                        id
                    }
                    None => {
                        if !self.cfg.signups_open {
                            return Err(Error::Forbidden("sign-ups are closed on this server".into()));
                        }
                        let name = match clean(&who.name).to_lowercase() {
                            n if n.is_empty() => email.split('@').next().unwrap_or("me").to_string(),
                            n => n,
                        };
                        sqlx::query_scalar("INSERT INTO users (name, email, password_hash, initials) VALUES ($1, $2, $3, $4) RETURNING id")
                            .bind(&name)
                            .bind(&email)
                            .bind(UNUSABLE_PASSWORD)
                            .bind(initials_of(&name))
                            .fetch_one(&mut *db)
                            .await?
                    }
                };
                sqlx::query("INSERT INTO identities (user_id, provider, subject, email) VALUES ($1, $2, $3, $4)")
                    .bind(id)
                    .bind(&who.provider)
                    .bind(&who.subject)
                    .bind(&email)
                    .execute(&mut *db)
                    .await?;
                id
            }
        };
        // a picture from the provider fills a gap; it never replaces one the person chose themselves
        if let Some(pic) = who.picture.as_deref().filter(|p| p.starts_with("data:image/") && p.len() <= 400_000) {
            sqlx::query("UPDATE users SET picture = $1 WHERE id = $2 AND picture IS NULL").bind(pic).bind(user_id).execute(&mut *db).await?;
        }
        let code = random("lgc_", 32);
        sqlx::query("DELETE FROM login_codes WHERE created_at < utc_text(now() - interval '2 minutes')").execute(&mut *db).await?;
        sqlx::query("INSERT INTO login_codes (code_hash, user_id) VALUES ($1, $2)").bind(hash(&code)).bind(user_id).execute(&mut *db).await?;
        db.commit().await?;
        Ok(code)
    }

    /// Add the provider `who` to the account `user_id`. It must not already belong to someone else.
    pub async fn link_identity(&self, user_id: i64, who: ExternalIdentity) -> Result<()> {
        if !PROVIDERS.contains(&who.provider.as_str()) {
            return Err(Error::NotFound("provider"));
        }
        let owner: Option<i64> = sqlx::query_scalar("SELECT user_id FROM identities WHERE provider = $1 AND subject = $2").bind(&who.provider).bind(&who.subject).fetch_optional(&self.pool).await?;
        match owner {
            Some(id) if id == user_id => return Ok(()),
            Some(_) => return Err(Error::Conflict(format!("that {} account is already used for another pebblelab account", who.provider))),
            None => {}
        }
        let taken: Option<i64> = sqlx::query_scalar("SELECT user_id FROM identities WHERE user_id = $1 AND provider = $2").bind(user_id).bind(&who.provider).fetch_optional(&self.pool).await?;
        if taken.is_some() {
            return Err(Error::Conflict(format!("you already have a {} account connected: disconnect it first", who.provider)));
        }
        sqlx::query("INSERT INTO identities (user_id, provider, subject, email) VALUES ($1, $2, $3, $4)").bind(user_id).bind(&who.provider).bind(&who.subject).bind(&who.email).execute(&self.pool).await?;
        Ok(())
    }

    /// Take a way to sign in off the account. The last one stays: without it nobody could get back in.
    pub async fn unlink_identity(&self, c: &Caller, provider: &str) -> Result<()> {
        if !c.is_interactive() {
            return Err(Error::Forbidden("only a signed-in person can remove a way to sign in".into()));
        }
        let mut db = self.pool.begin().await?;
        let have: Vec<String> = sqlx::query_scalar("SELECT provider FROM identities WHERE user_id = $1 FOR UPDATE").bind(c.user_id).fetch_all(&mut *db).await?;
        if !have.iter().any(|p| p == provider) {
            return Err(Error::NotFound("sign-in method"));
        }
        if have.len() < 2 {
            return Err(Error::bad("this is the only way you can sign in: connect another before removing it"));
        }
        sqlx::query("DELETE FROM identities WHERE user_id = $1 AND provider = $2").bind(c.user_id).bind(provider).execute(&mut *db).await?;
        db.commit().await?;
        Ok(())
    }

    /// A one-time code that an app (fin) swaps for its own session: how someone signed in here arrives there.
    pub async fn handoff(&self, c: &Caller) -> Result<String> {
        if !c.is_interactive() {
            return Err(Error::Forbidden("a token cannot open a session".into()));
        }
        let code = random("lgc_", 32);
        sqlx::query("DELETE FROM login_codes WHERE created_at < utc_text(now() - interval '2 minutes')").execute(&self.pool).await?;
        sqlx::query("INSERT INTO login_codes (code_hash, user_id) VALUES ($1, $2)").bind(hash(&code)).bind(c.user_id).execute(&self.pool).await?;
        Ok(code)
    }

    /// How the caller signs in, and where they are signed in. `token` is the one this request came with.
    pub async fn security(&self, c: &Caller, token: &str) -> Result<Security> {
        c.need_person("see where you are signed in")?;
        let me = crate::auth::hash_token(token);
        let methods = sqlx::query("SELECT provider, email, created_at FROM identities WHERE user_id = $1 ORDER BY provider")
            .bind(c.user_id)
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|r| SignInMethod { provider: r.get(0), email: r.get(1), created_at: r.get(2) })
            .collect();
        let sessions = sqlx::query("SELECT id, app, created_at, last_used_at, token_hash FROM tokens WHERE user_id = $1 AND kind = 'session' ORDER BY COALESCE(last_used_at, created_at) DESC")
            .bind(c.user_id)
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|r| SessionInfo { id: r.get(0), app: r.get(1), created_at: r.get(2), last_used_at: r.get(3), current: r.get::<String, _>(4) == me })
            .collect();
        Ok(Security { methods, sessions })
    }

    /// Sign out one of the caller's own sessions.
    pub async fn end_session(&self, c: &Caller, id: i64) -> Result<()> {
        c.need_person("sign you out")?;
        let n = sqlx::query("DELETE FROM tokens WHERE id = $1 AND user_id = $2 AND kind = 'session'").bind(id).bind(c.user_id).execute(&self.pool).await?.rows_affected();
        if n == 0 { Err(Error::NotFound("session")) } else { Ok(()) }
    }

    /// Swap a one-time code from a provider sign-in for a session.
    pub async fn redeem_login_code(&self, code: &str) -> Result<Session> {
        self.redeem_login_code_for(code, "").await
    }

    /// The same, noting which app the session is for.
    pub async fn redeem_login_code_for(&self, code: &str, app: &str) -> Result<Session> {
        let user_id: i64 = sqlx::query("DELETE FROM login_codes WHERE code_hash = $1 AND created_at > utc_text(now() - interval '2 minutes') RETURNING user_id")
            .bind(hash(code.trim()))
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::Unauthorized)?
            .get(0);
        self.open_session_for(user_id, if ["fin", "account"].contains(&app) { app } else { "" }).await
    }

    /// The providers this person has signed in with.
    pub async fn identities(&self, user_id: i64) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar("SELECT provider FROM identities WHERE user_id = $1 ORDER BY provider").bind(user_id).fetch_all(&self.pool).await?)
    }
}
