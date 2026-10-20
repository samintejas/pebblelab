//! Budgets: a monthly limit on a tag, and what the month has used of it.

use pebblelab_core::api::*;
use pebblelab_core::{Caller, Error, Store};

async fn user(s: &Store, name: &str) -> Caller {
    let sess = s.sign_up(SignUp { name: name.into(), email: format!("{name}@x.example"), password: "correct horse battery".into() }).await.unwrap();
    s.authenticate(&sess.token).await.unwrap()
}

fn bank(name: &str, visibility: Visibility) -> NewAccount {
    NewAccount { name: name.into(), kind: AccountKind::Bank, asset_id: None, balance: Some(1_000_000), visibility, owner_ids: vec![], details: AccountDetails::default() }
}

fn tx(account_id: i64, kind: TxKind, amount: i64, date: &str, tags: &[&str]) -> NewTransaction {
    NewTransaction { account_id, kind, amount, date: Some(date.into()), description: "x".into(), tags: tags.iter().map(|t| t.to_string()).collect(), party: String::new(), note: String::new() }
}

#[tokio::test]
async fn a_budget_counts_the_tag_this_month_less_refunds_and_never_transfers() {
    let s = Store::test().await.unwrap();
    let a = user(&s, "anita").await;
    let acct = s.create_account(&a, bank("salary", Visibility::Private)).await.unwrap();
    let card = s.create_account(&a, NewAccount { kind: AccountKind::Credit, ..bank("card", Visibility::Private) }).await.unwrap();
    let this = chrono::Utc::now().date_naive();
    let month = this.format("%Y-%m").to_string();
    let day = this.format("%Y-%m-%d").to_string();
    let last = (this - chrono::Months::new(1)).format("%Y-%m-15").to_string();
    let older = (this - chrono::Months::new(2)).format("%Y-%m-15").to_string();

    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 3_000_00, &day, &["groceries", "household"])).await.unwrap();
    s.add_transaction(&a, tx(card.id, TxKind::Debit, 1_500_00, &day, &["household", "groceries"])).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Credit, 500_00, &day, &["groceries"])).await.unwrap(); // a refund
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 900_00, &day, &["dining"])).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 2_000_00, &last, &["groceries"])).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 7_000_00, &older, &["groceries"])).await.unwrap();
    s.transfer(&a, NewTransfer { from_account_id: acct.id, to_account_id: card.id, amount: 10_000_00, date: Some(day.clone()), description: String::new(), note: String::new(), tags: vec!["groceries".into()] })
        .await
        .unwrap();

    let b = s.set_budget(&a, SetBudget { tag: " Groceries ".into(), amount: 5_000_00 }).await.unwrap();
    assert_eq!((b.tag.as_str(), b.amount, b.month.as_str()), ("groceries", 5_000_00, month.as_str()));
    assert_eq!(b.spent, 4_000_00, "3000 + 1500 on the card - 500 back; the transfer does not count");
    assert_eq!(b.last_month, 2_000_00);

    // setting it again changes the amount, it does not add a second
    s.set_budget(&a, SetBudget { tag: "groceries".into(), amount: 6_000_00 }).await.unwrap();
    let all = s.budgets(&a, BudgetQuery::default()).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].amount, 6_000_00);

    // another month
    let then = s.budgets(&a, BudgetQuery { month: Some(last[..7].to_string()) }).await.unwrap();
    assert_eq!((then[0].spent, then[0].last_month), (2_000_00, 7_000_00));
    assert!(matches!(s.budgets(&a, BudgetQuery { month: Some("2026-13".into()) }).await, Err(Error::BadRequest(_))));

    assert!(matches!(s.set_budget(&a, SetBudget { tag: "  ".into(), amount: 1 }).await, Err(Error::BadRequest(_))));
    assert!(matches!(s.set_budget(&a, SetBudget { tag: "x".into(), amount: 0 }).await, Err(Error::BadRequest(_))));

    s.delete_budget(&a, b.id).await.unwrap();
    assert!(s.budgets(&a, BudgetQuery::default()).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_budget_sees_what_the_family_shares_and_is_the_setters_own() {
    let s = Store::test().await.unwrap();
    let a = user(&s, "anita").await;
    let v = user(&s, "vikram").await;
    let x = user(&s, "stranger").await;
    s.create_family(&a, NewFamily { name: "rao".into() }).await.unwrap();
    let code = s.generate_invite(&a).await.unwrap().invite_code.unwrap();
    s.join_family(&v, JoinFamily { code }).await.unwrap();
    let today = chrono::Utc::now().date_naive().to_string();
    let shared = s.create_account(&v, bank("household", Visibility::Shared)).await.unwrap();
    let private = s.create_account(&v, bank("vikram's", Visibility::Private)).await.unwrap();
    let theirs = s.create_account(&x, bank("stranger's", Visibility::Shared)).await.unwrap();
    s.add_transaction(&v, tx(shared.id, TxKind::Debit, 1_000_00, &today, &["groceries"])).await.unwrap();
    s.add_transaction(&v, tx(private.id, TxKind::Debit, 400_00, &today, &["groceries"])).await.unwrap();
    s.add_transaction(&x, tx(theirs.id, TxKind::Debit, 9_999_00, &today, &["groceries"])).await.unwrap();

    let b = s.set_budget(&a, SetBudget { tag: "groceries".into(), amount: 5_000_00 }).await.unwrap();
    assert_eq!(b.spent, 1_000_00, "the shared account counts; vikram's private one and a stranger's do not");

    // nobody else sees it, or can remove it
    assert!(s.budgets(&v, BudgetQuery::default()).await.unwrap().is_empty());
    assert!(matches!(s.delete_budget(&x, b.id).await, Err(Error::NotFound(_))));

    // a token needs add to set one and edit to delete one
    let made = s.create_connector(&a, NewConnector { name: "r".into(), scopes: vec!["read".into()] }).await.unwrap();
    let t = s.authenticate(&made.token).await.unwrap();
    assert_eq!(s.budgets(&t, BudgetQuery::default()).await.unwrap().len(), 1);
    assert!(matches!(s.set_budget(&t, SetBudget { tag: "fuel".into(), amount: 1 }).await, Err(Error::Forbidden(_))));
    assert!(matches!(s.delete_budget(&t, b.id).await, Err(Error::Forbidden(_))));
}
