//! The monthly report: a month against the one before, its biggest spends, its budgets and its net worth.

use pebblelab_core::api::*;
use pebblelab_core::{Caller, Error, Store};

async fn user(s: &Store, name: &str) -> Caller {
    let sess = s.sign_up(SignUp { name: name.into(), email: format!("{name}@x.example"), password: "correct horse battery".into() }).await.unwrap();
    s.authenticate(&sess.token).await.unwrap()
}

fn tx(account_id: i64, kind: TxKind, amount: i64, date: &str, tags: &[&str], description: &str) -> NewTransaction {
    NewTransaction { account_id, kind, amount, date: Some(date.into()), description: description.into(), tags: tags.iter().map(|t| t.to_string()).collect(), party: String::new(), note: String::new() }
}

#[tokio::test]
async fn a_report_compares_the_month_with_the_one_before() {
    let s = Store::test().await.unwrap();
    let a = user(&s, "anita").await;
    let x = user(&s, "stranger").await;
    let bank = |name: &str| NewAccount { name: name.into(), kind: AccountKind::Bank, asset_id: None, balance: Some(0), visibility: Visibility::Private, owner_ids: vec![], details: AccountDetails::default() };
    let acct = s.create_account(&a, bank("salary")).await.unwrap();
    let savings = s.create_account(&a, bank("savings")).await.unwrap();
    let theirs = s.create_account(&x, bank("stranger's")).await.unwrap();
    let this = chrono::Utc::now().date_naive();
    let month = this.format("%Y-%m").to_string();
    let day = this.format("%Y-%m-01").to_string();
    let last = (this - chrono::Months::new(1)).format("%Y-%m-10").to_string();

    s.add_transaction(&a, tx(acct.id, TxKind::Credit, 100_000_00, &day, &["salary"], "pay")).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 30_000_00, &day, &["rent"], "rent")).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 4_000_00, &day, &["groceries"], "market")).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 50_000_00, &day, &["investment"], "sip")).await.unwrap(); // not spending
    for i in 1..=5 {
        s.add_transaction(&a, tx(acct.id, TxKind::Debit, i * 100_00, &day, &["dining"], &format!("meal {i}"))).await.unwrap();
    }
    s.transfer(&a, NewTransfer { from_account_id: acct.id, to_account_id: savings.id, amount: 5_000_00, date: Some(day.clone()), description: String::new(), note: String::new(), tags: vec![] }).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Credit, 90_000_00, &last, &["salary"], "pay")).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 6_000_00, &last, &["groceries"], "market")).await.unwrap();
    s.add_transaction(&a, tx(acct.id, TxKind::Debit, 2_000_00, &last, &["travel"], "train")).await.unwrap();
    s.add_transaction(&x, tx(theirs.id, TxKind::Debit, 99_000_00, &day, &["rent"], "not yours")).await.unwrap();
    s.set_budget(&a, SetBudget { tag: "groceries".into(), amount: 5_000_00 }).await.unwrap();

    let r = s.report(&a, ReportQuery::default()).await.unwrap();
    assert_eq!(r.month, month);
    assert_eq!((r.income, r.spending), (100_000_00, 30_000_00 + 4_000_00 + 15_00_00));
    assert_eq!((r.last_income, r.last_spending), (90_000_00, 8_000_00));
    let cats: Vec<(&str, i64, i64)> = r.categories.iter().map(|c| (c.tag.as_str(), c.total, c.last_month)).collect();
    assert_eq!(cats, vec![("rent", 30_000_00, 0), ("groceries", 4_000_00, 6_000_00), ("dining", 15_00_00, 0), ("travel", 0, 2_000_00)]);
    let big: Vec<(&str, i64)> = r.biggest.iter().map(|b| (b.description.as_str(), b.amount)).collect();
    assert_eq!(big, vec![("rent", 30_000_00), ("market", 4_000_00), ("meal 5", 500_00), ("meal 4", 400_00), ("meal 3", 300_00)], "the sip is not spending");
    assert_eq!(r.biggest[0].account, "salary");
    assert_eq!((r.budgets.len(), r.budgets[0].spent), (1, 4_000_00));
    let (start, end) = (r.net_start.unwrap(), r.net_end.unwrap());
    assert_eq!(end.own - end.owe, 100_000_00 - 30_000_00 - 4_000_00 - 50_000_00 - 15_00_00 + 90_000_00 - 8_000_00);
    assert_eq!(start.own - start.owe, 90_000_00 - 8_000_00);

    // last month on its own: the month before it had nothing
    let then = s.report(&a, ReportQuery { month: Some(last[..7].to_string()) }).await.unwrap();
    assert_eq!((then.income, then.spending, then.last_spending), (90_000_00, 8_000_00, 0));
    assert_eq!(then.budgets[0].spent, 6_000_00);

    let next = (this + chrono::Months::new(1)).format("%Y-%m").to_string();
    assert!(matches!(s.report(&a, ReportQuery { month: Some(next) }).await, Err(Error::BadRequest(_))));

    // a stranger's report holds none of it
    let theirs = s.report(&x, ReportQuery::default()).await.unwrap();
    assert_eq!((theirs.income, theirs.spending, theirs.biggest.len()), (0, 99_000_00, 1));
}
