use pebblelab_core::api::*;
use pebblelab_core::{Caller, Error, Store};

fn acct(name: &str, kind: AccountKind, balance: i64) -> NewAccount {
    NewAccount {
        name: name.into(),
        kind,
        asset_id: None,
        balance: Some(balance),
        visibility: Visibility::Private,
        owner_ids: vec![],
        details: AccountDetails::default(),
    }
}

fn spend(account_id: i64, amount: i64, date: &str) -> NewTransaction {
    NewTransaction {
        account_id,
        kind: TxKind::Debit,
        amount,
        date: Some(date.into()),
        description: "groceries".into(),
        tags: vec!["food".into()],
        party: String::new(),
        note: String::new(),
    }
}

fn through(date: &str, balance: Option<i64>) -> Reconcile {
    Reconcile { through: date.into(), balance }
}

async fn user(s: &Store, name: &str) -> Caller {
    let sess = s.sign_up(SignUp { name: name.into(), email: format!("{name}@x.example"), password: "correct horse battery".into() }).await.unwrap();
    s.authenticate(&sess.token).await.unwrap()
}

fn refused<T: std::fmt::Debug>(r: Result<T, Error>, what: &str) {
    match r {
        Err(Error::BadRequest(m)) => assert!(m.contains("reconciled through 2026-01-31"), "{what}: {m}"),
        other => panic!("{what}: expected a refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn a_reconciled_period_is_closed() {
    let s = Store::test().await.unwrap();
    let ana = user(&s, "anita").await;
    let bank = s.create_account(&ana, acct("salary", AccountKind::Bank, 1_000_00)).await.unwrap();
    let old = s.add_transaction(&ana, spend(bank.id, 100_00, "2026-01-10")).await.unwrap();
    let open = s.add_transaction(&ana, spend(bank.id, 10_00, "2026-02-03")).await.unwrap();

    // the statement must agree with the transactions up to its date
    match s.reconcile_account(&ana, bank.id, through("2026-01-31", Some(950_00))).await {
        Err(Error::BadRequest(m)) => assert!(m.contains("come to ₹900,") && m.contains("says ₹950:") && m.contains("₹50 less"), "{m}"),
        other => panic!("a wrong statement balance went through: {other:?}"),
    }
    assert!(s.account(&ana, bank.id).await.unwrap().reconciled.is_none(), "nothing saved when it does not match");
    let a = s.reconcile_account(&ana, bank.id, through("2026-01-31", Some(900_00))).await.unwrap();
    assert_eq!(a.reconciled, Some(Reconciled { through: "2026-01-31".into(), balance: 900_00 }));

    refused(s.add_transaction(&ana, spend(bank.id, 5_00, "2026-01-31")).await, "adding on the date");
    refused(s.update_transaction(&ana, old.id, UpdateTransaction { amount: Some(1_00), ..Default::default() }).await, "changing an amount");
    refused(s.update_transaction(&ana, old.id, UpdateTransaction { date: Some("2026-02-10".into()), ..Default::default() }).await, "moving one out");
    refused(s.update_transaction(&ana, open.id, UpdateTransaction { date: Some("2026-01-20".into()), ..Default::default() }).await, "moving one in");
    refused(s.update_transaction(&ana, old.id, UpdateTransaction { kind: Some(TxKind::Credit), ..Default::default() }).await, "flipping money out to in");
    refused(s.delete_transaction(&ana, old.id).await, "deleting");

    // what does not touch the balance stays editable
    let t = s
        .update_transaction(&ana, old.id, UpdateTransaction { description: Some("big shop".into()), tags: Some(vec!["household".into()]), note: Some("receipt in the drawer".into()), ..Default::default() })
        .await
        .unwrap();
    assert_eq!((t.description.as_str(), t.tags[0].as_str()), ("big shop", "household"));

    // after the date everything works as before
    s.add_transaction(&ana, spend(bank.id, 5_00, "2026-02-01")).await.unwrap();
    s.update_transaction(&ana, open.id, UpdateTransaction { amount: Some(12_00), ..Default::default() }).await.unwrap();
    assert_eq!(s.account(&ana, bank.id).await.unwrap().balance, 1_000_00 - 100_00 - 12_00 - 5_00);

    // clearing it opens the dates again
    assert!(s.clear_reconciliation(&ana, bank.id).await.unwrap().reconciled.is_none());
    s.update_transaction(&ana, old.id, UpdateTransaction { amount: Some(90_00), ..Default::default() }).await.unwrap();
}

#[tokio::test]
async fn both_legs_of_a_transfer_are_checked() {
    let s = Store::test().await.unwrap();
    let ana = user(&s, "anita").await;
    let bank = s.create_account(&ana, acct("salary", AccountKind::Bank, 1_000_00)).await.unwrap();
    let card = s.create_account(&ana, acct("card", AccountKind::Credit, 300_00)).await.unwrap();
    let a = s.reconcile_account(&ana, card.id, through("2026-01-31", Some(300_00))).await.unwrap();
    assert_eq!(a.reconciled.unwrap().balance, 300_00, "a card's figure is what is owed");
    let pay = |date: &str| NewTransfer { from_account_id: bank.id, to_account_id: card.id, amount: 50_00, date: Some(date.into()), description: String::new(), tags: vec![], note: String::new() };
    refused(s.transfer(&ana, pay("2026-01-20")).await, "paying the card on a closed date");
    let legs = s.transfer(&ana, pay("2026-02-02")).await.unwrap();
    refused(s.update_transaction(&ana, legs[0].id, UpdateTransaction { date: Some("2026-01-25".into()), ..Default::default() }).await, "moving the bank leg moves the card leg");
    assert_eq!(s.account(&ana, bank.id).await.unwrap().balance, 950_00, "nothing half done");
}

#[tokio::test]
async fn setting_the_balance_of_a_reconciled_account_adds_an_adjustment() {
    let s = Store::test().await.unwrap();
    let ana = user(&s, "anita").await;
    let bank = s.create_account(&ana, acct("salary", AccountKind::Bank, 1_000_00)).await.unwrap();
    s.add_transaction(&ana, spend(bank.id, 100_00, "2026-01-10")).await.unwrap();
    s.reconcile_account(&ana, bank.id, through("2026-01-31", None)).await.unwrap();
    let a = s.update_account(&ana, bank.id, UpdateAccount { balance: Some(870_00), ..Default::default() }).await.unwrap();
    assert_eq!(a.balance, 870_00);
    assert_eq!(a.reconciled.unwrap().balance, 900_00, "the reconciled figure stays");
    let page = s.transactions(&ana, TxFilter { account_id: Some(bank.id), tags: Some("adjustment".into()), ..Default::default() }).await.unwrap();
    assert_eq!(page.total, 1);
    assert_eq!((page.items[0].amount, page.items[0].kind), (-30_00, TxKind::Debit));
    // the same balance again changes nothing
    s.update_account(&ana, bank.id, UpdateAccount { balance: Some(870_00), ..Default::default() }).await.unwrap();
    assert_eq!(s.transactions(&ana, TxFilter { account_id: Some(bank.id), ..Default::default() }).await.unwrap().total, 2);
}

#[tokio::test]
async fn what_can_be_reconciled() {
    let s = Store::test().await.unwrap();
    let ana = user(&s, "anita").await;
    let ben = user(&s, "ben").await;
    let bank = s.create_account(&ana, acct("salary", AccountKind::Bank, 1_000_00)).await.unwrap();
    assert!(matches!(s.reconcile_account(&ben, bank.id, through("2026-01-31", None)).await, Err(Error::NotFound(_))), "someone else's account");
    assert!(matches!(s.reconcile_account(&ana, bank.id, through("2999-01-01", None)).await, Err(Error::BadRequest(_))), "a future statement");
    assert!(matches!(s.reconcile_account(&ana, bank.id, through("", None)).await, Err(Error::BadRequest(_))), "no date");
    let mut loan = acct("home", AccountKind::Loan, 0);
    loan.details = AccountDetails { loan_total: Some(10_00_000_00), rate: Some(8.5), tenure: Some(240), start: Some("2024-01".into()), ..Default::default() };
    let loan = s.create_account(&ana, loan).await.unwrap();
    assert!(matches!(s.reconcile_account(&ana, loan.id, through("2026-01-31", None)).await, Err(Error::BadRequest(_))), "a loan follows its schedule");
    // a reconciled account can still be deleted, with its transactions
    s.add_transaction(&ana, spend(bank.id, 100_00, "2026-01-10")).await.unwrap();
    s.reconcile_account(&ana, bank.id, through("2026-01-31", None)).await.unwrap();
    s.delete_account(&ana, bank.id).await.unwrap();
}

#[tokio::test]
async fn renewals_skip_reconciled_dates() {
    let s = Store::test().await.unwrap();
    let ana = user(&s, "anita").await;
    let bank = s.create_account(&ana, acct("salary", AccountKind::Bank, 1_000_00)).await.unwrap();
    s.reconcile_account(&ana, bank.id, through("2026-01-31", None)).await.unwrap();
    s.add_subscription(&ana, NewSubscription { name: "music".into(), amount: 1_00, cycle: Cycle::Monthly, next: Some("2026-01-05".into()), account_id: bank.id, tag: "fun".into(), active: true })
        .await
        .unwrap();
    let page = s.transactions(&ana, TxFilter { account_id: Some(bank.id), ..Default::default() }).await.unwrap();
    assert!(page.total > 0, "the later renewals are there");
    assert!(page.items.iter().all(|t| t.date.as_str() > "2026-01-31"), "{:?}", page.items.iter().map(|t| &t.date).collect::<Vec<_>>());
}
