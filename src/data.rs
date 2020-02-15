use std::time::SystemTime;

use crate::{Money, Debt};


pub fn get_users() -> Vec<String> {
    vec!["alice".into(),"bob".into(), "ben".into(), "mitchell".into()]
}
pub fn get_all_debts() -> Vec<Debt> {
    vec![]
}

pub fn get_debts_involving(user: &str) -> Vec<Debt> {
    vec![
        Debt{ debtor: "asdf".into(), creditor: "asdf".into(), time: SystemTime::now(), amount: Money::from_dollars(1) },
        Debt{ debtor: "asdf".into(), creditor: "asdf".into(), time: SystemTime::now(), amount: Money::from_dollars(1) },
        Debt{ debtor: "asdf".into(), creditor: "asdf".into(), time: SystemTime::now(), amount: Money::from_dollars(1) },
        Debt{ debtor: "asdf".into(), creditor: "asdf".into(), time: SystemTime::now(), amount: Money::from_dollars(1) }]
}
