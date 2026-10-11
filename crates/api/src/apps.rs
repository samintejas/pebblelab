//! pebblelab is one account and a set of small apps. The account (who you are, how you sign in, your family,
//! your tokens) is shared; each app keeps its own data and lives at its own address.

use serde::{Deserialize, Serialize};

/// An app under the pebblelab account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppInfo {
    /// Stable id: in scopes (`fin:read`), sessions and configuration.
    pub id: &'static str,
    /// Shown in the app switcher, lowercase.
    pub name: &'static str,
    /// One line about it.
    pub summary: &'static str,
    /// A dots icon name for its tile.
    pub icon: &'static str,
}

/// The account itself. Its sessions may do anything, in every app.
pub const ACCOUNT: AppInfo = AppInfo { id: "account", name: "account", summary: "who you are, how you sign in, your family and your tokens.", icon: "user" };

/// The household money tracker.
pub const FIN: AppInfo = AppInfo { id: "fin", name: "fin", summary: "a household money tracker.", icon: "wallet" };

/// Every app, the account first.
pub const APPS: [AppInfo; 2] = [ACCOUNT, FIN];

/// The app with this id.
pub fn app(id: &str) -> Option<AppInfo> {
    APPS.into_iter().find(|a| a.id == id)
}

/// What a connector token may be given: an app and an action, `fin:read`.
pub const SCOPES: [&str; 4] = [FIN_READ, FIN_TRANSACTIONS, FIN_ADD, FIN_EDIT];

/// fin: read accounts, balances, insights, subscriptions and assets.
pub const FIN_READ: &str = "fin:read";
/// fin: read transactions.
pub const FIN_TRANSACTIONS: &str = "fin:transactions";
/// fin: add transactions, subscriptions and assets.
pub const FIN_ADD: &str = "fin:add";
/// fin: change or delete anything.
pub const FIN_EDIT: &str = "fin:edit";

/// A scope as stored: `read` from before scopes named their app is `fin:read`. `None` for an unknown one.
pub fn scope(s: &str) -> Option<&'static str> {
    let s = s.trim().to_lowercase();
    let full = if s.contains(':') { s } else { format!("fin:{s}") };
    SCOPES.into_iter().find(|x| *x == full)
}

/// The scopes a session in `app` holds. A session in the account holds every app's.
pub fn session_scopes(app: &str) -> Vec<String> {
    SCOPES.iter().filter(|s| app == ACCOUNT.id || s.starts_with(&format!("{app}:"))).map(|s| s.to_string()).collect()
}

/// One app as this server runs it: where it lives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteApp {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub icon: String,
    /// Its public address, `https://fin.pebblelab.in`, no trailing slash.
    pub url: String,
}

/// What the web app needs to know about where it is running.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Site {
    /// Which app this address serves: `account`, `fin`, or `all` when one address serves them all.
    pub here: String,
    /// Where the account lives: sign-in, profile, family, tokens.
    pub account_url: String,
    pub apps: Vec<SiteApp>,
}

impl Site {
    /// The address of an app.
    pub fn url(&self, id: &str) -> Option<&str> {
        self.apps.iter().find(|a| a.id == id).map(|a| a.url.as_str())
    }

    /// One address serves everything (a single-host install, or development).
    pub fn single(&self) -> bool {
        self.here == "all"
    }
}

/// A signed-in browser, for the security page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: i64,
    /// `account`, or the app it was opened for.
    pub app: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    /// The session making this request.
    pub current: bool,
}

/// Ask the account to sign you in to an app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRequest {
    pub app: String,
}

/// Where to send the browser: the app's own sign-in callback, with a one-time code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handoff {
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_scope_names_mean_fin() {
        assert_eq!(scope("read"), Some(FIN_READ));
        assert_eq!(scope("fin:edit"), Some(FIN_EDIT));
        assert_eq!(scope(" Transactions "), Some(FIN_TRANSACTIONS));
        assert_eq!(scope("notes:read"), None);
        assert_eq!(scope("admin"), None);
    }

    #[test]
    fn sessions_hold_their_apps_scopes() {
        assert_eq!(session_scopes("fin").len(), 4);
        assert_eq!(session_scopes("account").len(), 4);
        assert!(session_scopes("notes").is_empty());
    }
}
