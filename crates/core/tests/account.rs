//! The pebblelab account over its apps: one sign-in, handed to fin; fin sessions that cannot change the
//! account; tokens whose scopes name their app.

use pebblelab_core::api::*;
use pebblelab_core::{Error, Store};

async fn sign_up(s: &Store, name: &str) -> Session {
    s.sign_up(SignUp { name: name.into(), email: format!("{name}@x.example"), password: "correct horse battery".into() }).await.unwrap()
}

#[tokio::test]
async fn the_account_hands_a_browser_to_fin() {
    let s = Store::test().await.unwrap();
    let acct = sign_up(&s, "anita").await;
    let a = s.authenticate(&acct.token).await.unwrap();
    assert!(a.is_account(), "signing in is signing in to the account");

    let code = s.handoff(&a, "fin").await.unwrap();
    let fin = s.redeem_login_code(&code).await.unwrap();
    assert_ne!(fin.token, acct.token, "fin gets a session of its own");
    assert!(matches!(s.redeem_login_code(&code).await, Err(Error::Unauthorized)), "a code works once");

    let f = s.authenticate(&fin.token).await.unwrap();
    assert!(!f.is_account());
    assert!(f.can(FIN_READ) && f.can(FIN_EDIT));
    // fin can use fin and read who you are
    s.create_account(&f, NewAccount { name: "salary".into(), kind: AccountKind::Bank, asset_id: None, balance: Some(100), visibility: Visibility::Private, owner_ids: vec![], details: AccountDetails::default() }).await.unwrap();
    assert_eq!(s.me(&f).await.unwrap().user.name, "anita");
    // and keep its own preferences
    s.update_profile(&f, UpdateProfile { currency: Some("usd".into()), ..Default::default() }).await.unwrap();
    // but the account is changed only in the account
    assert!(matches!(s.update_profile(&f, UpdateProfile { name: Some("someone".into()), ..Default::default() }).await, Err(Error::Forbidden(_))));
    assert!(matches!(s.create_connector(&f, NewConnector { name: "claude".into(), scopes: vec![] }).await, Err(Error::Forbidden(_))));
    assert!(matches!(s.create_family(&f, NewFamily { name: "rao".into() }).await, Err(Error::Forbidden(_))));
    assert!(matches!(s.handoff(&f, "fin").await, Err(Error::Forbidden(_))), "an app cannot hand itself on");
    assert!(matches!(s.sign_out_all(&f).await, Err(Error::Forbidden(_))));

    // the account sees both sessions, and which one is asking
    let list = s.sessions(&a).await.unwrap();
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|x| x.app == "fin" && !x.current));
    assert!(list.iter().any(|x| x.app == "account" && x.current));
}

#[tokio::test]
async fn handing_off_needs_a_real_app() {
    let s = Store::test().await.unwrap();
    let a = s.authenticate(&sign_up(&s, "anita").await.token).await.unwrap();
    assert!(matches!(s.handoff(&a, "notes").await, Err(Error::NotFound(_))));
    assert!(matches!(s.handoff(&a, "account").await, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn signing_out_of_fin_signs_the_browser_out() {
    let s = Store::test().await.unwrap();
    let acct = sign_up(&s, "anita").await;
    let a = s.authenticate(&acct.token).await.unwrap();
    let fin = s.redeem_login_code(&s.handoff(&a, "fin").await.unwrap()).await.unwrap();
    // another browser, signed in separately
    let other = s.sign_in(SignIn { email: "anita@x.example".into(), password: "correct horse battery".into() }).await.unwrap();

    s.sign_out(&fin.token).await.unwrap();
    assert!(matches!(s.authenticate(&fin.token).await, Err(Error::Unauthorized)));
    assert!(matches!(s.authenticate(&acct.token).await, Err(Error::Unauthorized)), "the account session that opened it ends too");
    assert!(s.authenticate(&other.token).await.is_ok(), "other browsers stay signed in");
}

#[tokio::test]
async fn ending_an_account_session_ends_its_apps() {
    let s = Store::test().await.unwrap();
    let acct = sign_up(&s, "anita").await;
    let a = s.authenticate(&acct.token).await.unwrap();
    let fin = s.redeem_login_code(&s.handoff(&a, "fin").await.unwrap()).await.unwrap();
    let other = s.authenticate(&s.sign_in(SignIn { email: "anita@x.example".into(), password: "correct horse battery".into() }).await.unwrap().token).await.unwrap();
    // from another browser, end the first one
    s.revoke_session(&other, a.session().unwrap()).await.unwrap();
    assert!(matches!(s.authenticate(&fin.token).await, Err(Error::Unauthorized)));
    // someone else's session is not yours to end
    let b = s.authenticate(&sign_up(&s, "vikram").await.token).await.unwrap();
    assert!(matches!(s.revoke_session(&b, other.session().unwrap()).await, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn token_scopes_name_their_app() {
    let s = Store::test().await.unwrap();
    let a = s.authenticate(&sign_up(&s, "anita").await.token).await.unwrap();
    // the old names still work, and are stored with their app
    let made = s.create_connector(&a, NewConnector { name: "claude".into(), scopes: vec!["read".into(), "fin:add".into(), "read".into()] }).await.unwrap();
    assert_eq!(made.connector.scopes, vec![FIN_READ, FIN_ADD]);
    assert!(matches!(s.create_connector(&a, NewConnector { name: "x".into(), scopes: vec!["notes:read".into()] }).await, Err(Error::BadRequest(_))));
    let t = s.authenticate(&made.token).await.unwrap();
    assert!(t.can(FIN_READ) && !t.can(FIN_EDIT));
    assert!(matches!(s.update_profile(&t, UpdateProfile { name: Some("x".into()), ..Default::default() }).await, Err(Error::Forbidden(_))));
    assert!(matches!(s.handoff(&t, "fin").await, Err(Error::Forbidden(_))));
    // with no scopes given: read fin and its transactions
    let plain = s.create_connector(&a, NewConnector { name: "script".into(), scopes: vec![] }).await.unwrap();
    assert_eq!(plain.connector.scopes, vec![FIN_READ, FIN_TRANSACTIONS]);
}

#[tokio::test]
async fn tokens_from_before_apps_keep_working() {
    let s = Store::test().await.unwrap();
    let a = s.authenticate(&sign_up(&s, "anita").await.token).await.unwrap();
    let made = s.create_connector(&a, NewConnector { name: "old".into(), scopes: vec![] }).await.unwrap();
    // what 0008 found in the table: names without their app
    s.raw(&format!("UPDATE tokens SET scopes = 'read,transactions,add,edit' WHERE id = {}", made.connector.id)).await.unwrap();
    s.raw("UPDATE tokens SET scopes = regexp_replace(scopes, '(^|,)([a-z]+)(?=,|$)', '\\1fin:\\2', 'g') WHERE kind = 'connector'").await.unwrap();
    let t = s.authenticate(&made.token).await.unwrap();
    assert_eq!(t.scopes(), [FIN_READ, FIN_TRANSACTIONS, FIN_ADD, FIN_EDIT]);
}
