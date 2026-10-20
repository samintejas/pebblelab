//! Net worth over time: worked back from today through the transactions since.

use chrono::{Datelike, Months};
use pebblelab_core::api::*;
use pebblelab_core::{Caller, Store};

async fn user(s: &Store, name: &str) -> Caller {
    let sess = s.sign_up(SignUp { name: name.into(), email: format!("{name}@x.example"), password: "correct horse battery".into() }).await.unwrap();
    s.authenticate(&sess.token).await.unwrap()
}

fn acct(name: &str, kind: AccountKind, balance: i64) -> NewAccount {
    NewAccount { name: name.into(), kind, asset_id: None, balance: Some(balance), visibility: Visibility::Private, owner_ids: vec![], details: AccountDetails::default() }
}

fn spend(account_id: i64, amount: i64, date: &str) -> NewTransaction {
    NewTransaction { account_id, kind: TxKind::Debit, amount, date: Some(date.into()), description: "x".into(), tags: vec![], party: String::new(), note: String::new() }
}

#[tokio::test]
async fn net_worth_runs_back_from_today_through_the_transactions() {
    let s = Store::test().await.unwrap();
    let a = user(&s, "anita").await;
    let today = chrono::Utc::now().date_naive();
    let bank = s.create_account(&a, acct("salary", AccountKind::Bank, 100_000_00)).await.unwrap();
    let card = s.create_account(&a, acct("card", AccountKind::Credit, 0)).await.unwrap();
    // two months ago: 30,000 out of the bank and 5,000 on the card
    let then = (today - Months::new(2)).with_day(10).unwrap().to_string();
    s.add_transaction(&a, spend(bank.id, 30_000_00, &then)).await.unwrap();
    s.add_transaction(&a, spend(card.id, 5_000_00, &then)).await.unwrap();
    s.add_asset(&a, NewAsset { name: "car".into(), kind: "vehicle".into(), bought: (today - Months::new(1)).format("%Y-%m").to_string(), cost: None, value: 4_00_000_00, note: String::new() }).await.unwrap();

    let ins = s.insights(&a, InsightsQuery::default()).await.unwrap();
    let nw = &ins.net_worth;
    assert_eq!(nw.len(), 12);
    let last = nw.last().unwrap();
    assert_eq!(last.month, today.format("%Y-%m").to_string());
    assert_eq!((last.own, last.owe), (ins.assets, ins.owed), "today's point is today's insights");
    assert_eq!((last.own, last.owe), (70_000_00 + 4_00_000_00, 5_000_00));
    // three months ago nothing was tracked yet: an account counts from its first month (its first transaction here)
    let before = &nw[nw.len() - 4];
    assert_eq!((before.own, before.owe), (0, 0));
    // the month they happened: the transactions count, the car (bought a month later) does not
    let during = &nw[nw.len() - 3];
    assert_eq!((during.own, during.owe), (70_000_00, 5_000_00));
}
