//! Whose data is whose. A stranger finds nothing of yours by id, list or search; family sees what you share, and only
//! reads it; and a connector token works on the money it was given, never on the account itself.

use pebblelab_core::api::*;
use pebblelab_core::{Caller, Error, Store};

fn acct(name: &str, visibility: Visibility) -> NewAccount {
    NewAccount { name: name.into(), kind: AccountKind::Bank, asset_id: None, balance: Some(10_000_00), visibility, owner_ids: vec![], details: AccountDetails::default() }
}

fn spend(account_id: i64, desc: &str) -> NewTransaction {
    NewTransaction { account_id, kind: TxKind::Debit, amount: 250_00, date: None, description: desc.into(), tags: vec!["secretcategory".into()], party: "secretparty".into(), note: String::new() }
}

async fn user(s: &Store, name: &str) -> (Caller, User) {
    let sess = s.sign_up(SignUp { name: name.into(), email: format!("{name}@x.example"), password: "correct horse battery".into() }).await.unwrap();
    (s.authenticate(&sess.token).await.unwrap(), sess.user)
}

fn not_found<T: std::fmt::Debug>(r: Result<T, Error>, what: &str) {
    assert!(matches!(r, Err(Error::NotFound(_))), "{what}: expected not found, got {r:?}");
}

fn forbidden<T: std::fmt::Debug>(r: Result<T, Error>, what: &str) {
    assert!(matches!(r, Err(Error::Forbidden(_))), "{what}: expected forbidden, got {r:?}");
}

/// What anita has: a private and a family-shared account, a transaction with a file on each, a subscription, an asset.
struct Anita {
    c: Caller,
    private: Account,
    shared: Account,
    tx: Transaction,
    shared_tx: Transaction,
    file: Attachment,
    sub: Subscription,
    asset: Asset,
}

async fn anita(s: &Store) -> Anita {
    let (c, _) = user(s, "anita").await;
    let private = s.create_account(&c, acct("private savings", Visibility::Private)).await.unwrap();
    let shared = s.create_account(&c, acct("household", Visibility::Shared)).await.unwrap();
    let tx = s.add_transaction(&c, spend(private.id, "secret purchase")).await.unwrap();
    let shared_tx = s.add_transaction(&c, spend(shared.id, "groceries")).await.unwrap();
    let file = s.add_attachment(&c, tx.id, "receipt.pdf", "application/pdf", b"%PDF-1.4".to_vec()).await.unwrap();
    let sub = s
        .add_subscription(&c, NewSubscription { name: "music".into(), amount: 119_00, cycle: Cycle::default(), next: None, account_id: private.id, tag: String::new(), active: true })
        .await
        .unwrap();
    let asset = s.add_asset(&c, NewAsset { name: "flat".into(), kind: "property".into(), bought: String::new(), cost: None, value: 50_00_000_00, note: String::new() }).await.unwrap();
    Anita { c, private, shared, tx, shared_tx, file, sub, asset }
}

#[tokio::test]
async fn a_stranger_finds_nothing_of_yours_by_id_list_or_search() {
    let s = Store::test().await.unwrap();
    let a = anita(&s).await;
    let (x, _) = user(&s, "stranger").await;
    let mine = s.create_account(&x, acct("stranger's", Visibility::Shared)).await.unwrap();

    for id in [a.private.id, a.shared.id] {
        not_found(s.account(&x, id).await, "read an account");
        not_found(s.update_account(&x, id, UpdateAccount { name: Some("mine now".into()), ..Default::default() }).await, "rename an account");
        not_found(s.delete_account(&x, id).await, "delete an account");
        not_found(s.leave_account(&x, id).await, "leave an account");
        not_found(s.reconcile_account(&x, id, Reconcile { through: "2026-01-01".into(), balance: None }).await, "reconcile an account");
        not_found(s.clear_reconciliation(&x, id).await, "reopen an account");
        not_found(s.add_transaction(&x, spend(id, "into someone else's")).await, "add to an account");
        not_found(s.transfer(&x, NewTransfer { from_account_id: mine.id, to_account_id: id, amount: 1_00, date: None, description: String::new(), note: String::new(), tags: vec![] }).await, "transfer into an account");
        not_found(s.transfer(&x, NewTransfer { from_account_id: id, to_account_id: mine.id, amount: 1_00, date: None, description: String::new(), note: String::new(), tags: vec![] }).await, "transfer out of an account");
        let filtered = s.transactions(&x, TxFilter { account_id: Some(id), ..Default::default() }).await.unwrap();
        assert_eq!(filtered.total, 0, "a filter by someone else's account finds nothing");
        let listed = s.transactions(&x, TxFilter { accounts: Some(id.to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(listed.total, 0, "a list of someone else's accounts finds nothing");
        not_found(
            s.add_subscription(&x, NewSubscription { name: "theirs".into(), amount: 1_00, cycle: Cycle::default(), next: None, account_id: id, tag: String::new(), active: true }).await,
            "subscribe on an account",
        );
    }
    for id in [a.tx.id, a.shared_tx.id] {
        not_found(s.transaction(&x, id).await, "read a transaction");
        not_found(s.update_transaction(&x, id, UpdateTransaction { description: Some("x".into()), ..Default::default() }).await, "edit a transaction");
        not_found(s.delete_transaction(&x, id).await, "delete a transaction");
        not_found(s.add_attachment(&x, id, "x.pdf", "application/pdf", b"x".to_vec()).await, "attach to a transaction");
    }
    // moving your own transaction into their account
    let own = s.add_transaction(&x, NewTransaction { tags: vec![], party: String::new(), ..spend(mine.id, "mine") }).await.unwrap();
    not_found(s.update_transaction(&x, own.id, UpdateTransaction { account_id: Some(a.private.id), ..Default::default() }).await, "move a transaction into an account");
    not_found(s.attachment(&x, a.file.id).await, "read a file");
    not_found(s.delete_attachment(&x, a.file.id).await, "delete a file");
    not_found(s.update_subscription(&x, a.sub.id, UpdateSubscription { name: Some("x".into()), ..Default::default() }).await, "edit a subscription");
    not_found(s.delete_subscription(&x, a.sub.id).await, "delete a subscription");
    not_found(s.update_asset(&x, a.asset.id, UpdateAsset { value: Some(1), ..Default::default() }).await, "edit an asset");
    not_found(s.delete_asset(&x, a.asset.id).await, "delete an asset");

    // lists and searches
    assert_eq!(s.accounts(&x, true).await.unwrap().len(), 1);
    let found = s.transactions(&x, TxFilter { q: Some("secret".into()), ..Default::default() }).await.unwrap();
    assert_eq!(found.total, 0, "search finds nothing of theirs");
    let by_uid = s.transactions(&x, TxFilter { q: Some(a.tx.uid.clone()), ..Default::default() }).await.unwrap();
    assert_eq!(by_uid.total, 0, "nor by its id");
    assert!(s.tags(&x).await.unwrap().iter().all(|(t, _)| t != "secretcategory"));
    assert!(s.parties(&x).await.unwrap().iter().all(|(p, _)| p != "secretparty"));
    assert!(s.subscriptions(&x).await.unwrap().is_empty());
    assert!(s.assets(&x).await.unwrap().is_empty());
    assert!(s.notifications(&x).await.unwrap().is_empty());
    let csv = s.export_csv(&x).await.unwrap();
    assert!(!csv.contains("secret") && !csv.contains("household") && !csv.contains("groceries"), "the export holds only their own: {csv}");
    let ins = s.insights(&x, InsightsQuery { member_id: Some(a.c.user_id), days: None }).await.unwrap();
    assert!(ins.accounts.is_empty(), "insights about someone outside your family show nothing");
    // and anita's own view is untouched
    assert_eq!(s.accounts(&a.c, true).await.unwrap().len(), 2);
    assert_eq!(s.account(&a.c, a.private.id).await.unwrap().name, "private savings");
}

#[tokio::test]
async fn family_sees_what_you_share_and_only_reads_it() {
    let s = Store::test().await.unwrap();
    let a = anita(&s).await;
    let (v, _) = user(&s, "vikram").await;
    s.create_family(&a.c, NewFamily { name: "rao".into() }).await.unwrap();
    let code = s.generate_invite(&a.c).await.unwrap().invite_code.unwrap();
    s.join_family(&v, JoinFamily { code }).await.unwrap();

    // the shared account and its transactions show; the private one does not
    let seen: Vec<i64> = s.accounts(&v, true).await.unwrap().iter().map(|a| a.id).collect();
    assert_eq!(seen, vec![a.shared.id]);
    assert_eq!(s.transaction(&v, a.shared_tx.id).await.unwrap().id, a.shared_tx.id);
    not_found(s.transaction(&v, a.tx.id).await, "a private transaction");
    not_found(s.attachment(&v, a.file.id).await, "a file on a private transaction");
    assert_eq!(s.transactions(&v, TxFilter::default()).await.unwrap().total, 1);

    // and none of it can be changed by someone who does not own it
    forbidden(s.update_account(&v, a.shared.id, UpdateAccount { name: Some("ours".into()), ..Default::default() }).await, "rename");
    forbidden(s.update_account(&v, a.shared.id, UpdateAccount { visibility: Some(Visibility::Private), ..Default::default() }).await, "hide it from the owner");
    forbidden(s.update_account(&v, a.shared.id, UpdateAccount { owner_ids: Some(vec![v.user_id]), ..Default::default() }).await, "make themselves an owner");
    forbidden(s.delete_account(&v, a.shared.id).await, "delete");
    forbidden(s.reconcile_account(&v, a.shared.id, Reconcile { through: "2026-01-01".into(), balance: None }).await, "reconcile");
    forbidden(s.add_transaction(&v, spend(a.shared.id, "on their account")).await, "add");
    forbidden(s.update_transaction(&v, a.shared_tx.id, UpdateTransaction { amount: Some(1_00), ..Default::default() }).await, "edit");
    forbidden(s.delete_transaction(&v, a.shared_tx.id).await, "delete a transaction");
    forbidden(s.add_attachment(&v, a.shared_tx.id, "x.pdf", "application/pdf", b"x".to_vec()).await, "attach");

    // once vikram leaves, the shared account goes too
    s.leave_family(&v).await.unwrap();
    assert!(s.accounts(&v, true).await.unwrap().is_empty());
    not_found(s.transaction(&v, a.shared_tx.id).await, "after leaving");
}

#[tokio::test]
async fn a_connector_works_on_the_money_but_never_on_the_account() {
    let s = Store::test().await.unwrap();
    let a = anita(&s).await;
    let all = SCOPES.iter().map(|s| s.to_string()).collect();
    let made = s.create_connector(&a.c, NewConnector { name: "claude".into(), scopes: all }).await.unwrap();
    let t = s.authenticate(&made.token).await.unwrap();
    assert!(!t.is_interactive());

    // the money: yes
    s.add_transaction(&t, spend(a.private.id, "lunch")).await.unwrap();
    s.update_account(&t, a.private.id, UpdateAccount { name: Some("savings".into()), ..Default::default() }).await.unwrap();
    s.update_account(&t, a.shared.id, UpdateAccount { visibility: Some(Visibility::Shared), ..Default::default() }).await.unwrap();
    s.reconcile_account(&t, a.private.id, Reconcile { through: "2020-01-01".into(), balance: None }).await.unwrap();

    // the account, who sees what, and the tokens: no
    forbidden(s.update_profile(&t, UpdateProfile { email: Some("taken@evil.example".into()), ..Default::default() }).await, "change the email");
    forbidden(s.change_password(&t, ChangePassword { current: "correct horse battery".into(), new: "a brand new passphrase".into() }).await, "change the password");
    forbidden(s.delete_user(&t, "correct horse battery").await, "delete the account");
    forbidden(s.create_connector(&t, NewConnector { name: "more".into(), scopes: vec!["read".into()] }).await, "make another token");
    forbidden(s.connectors(&t).await, "list tokens");
    forbidden(s.update_connector(&t, made.connector.id, UpdateConnector { scopes: vec!["read".into()] }).await, "change a token");
    forbidden(s.revoke_connector(&t, made.connector.id).await, "remove a token");
    forbidden(s.sign_out_all(&t).await, "sign out everywhere");
    forbidden(s.security(&t, &made.token).await, "see sessions");
    forbidden(s.end_session(&t, 1).await, "end a session");
    forbidden(s.create_family(&t, NewFamily { name: "x".into() }).await, "create a family");
    forbidden(s.join_family(&t, JoinFamily { code: "ABCDEF".into() }).await, "join a family");
    forbidden(s.leave_family(&t).await, "leave a family");
    forbidden(s.update_account(&t, a.private.id, UpdateAccount { visibility: Some(Visibility::Shared), ..Default::default() }).await, "share an account");
    forbidden(s.update_account(&t, a.shared.id, UpdateAccount { visibility: Some(Visibility::Private), ..Default::default() }).await, "unshare an account");
    forbidden(s.update_account(&t, a.private.id, UpdateAccount { owner_ids: Some(vec![a.c.user_id + 1]), ..Default::default() }).await, "add an owner");
    forbidden(s.delete_account(&t, a.private.id).await, "delete an account");
    forbidden(s.clear_reconciliation(&t, a.private.id).await, "reopen reconciled dates");

    // nothing of that changed
    let me = s.me(&a.c).await.unwrap();
    assert_eq!(me.user.email, "anita@x.example");
    assert!(me.family.is_none());
    assert_eq!(s.connectors(&a.c).await.unwrap().len(), 1);
    assert_eq!(s.account(&a.c, a.private.id).await.unwrap().visibility, Visibility::Private);
    assert!(s.account(&a.c, a.private.id).await.unwrap().reconciled.is_some());
}

#[tokio::test]
async fn a_token_reads_only_what_its_scopes_allow() {
    let s = Store::test().await.unwrap();
    let a = anita(&s).await;
    let made = s.create_connector(&a.c, NewConnector { name: "reader".into(), scopes: vec!["read".into()] }).await.unwrap();
    let t = s.authenticate(&made.token).await.unwrap();
    s.accounts(&t, false).await.unwrap();
    forbidden(s.transactions(&t, TxFilter::default()).await, "list transactions");
    forbidden(s.transaction(&t, a.tx.id).await, "read a transaction");
    forbidden(s.attachment(&t, a.file.id).await, "read a file");
    forbidden(s.export_csv(&t).await, "export");
    forbidden(s.add_transaction(&t, spend(a.private.id, "x")).await, "add");
    forbidden(s.update_account(&t, a.private.id, UpdateAccount { name: Some("x".into()), ..Default::default() }).await, "edit");
}
