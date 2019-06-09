#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::State;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use sled::Db;
use bincode::{serialize, deserialize};

fn get<E: DeserializeOwned>(db: &Db, key: &str) -> Result<Option<E>, Box<dyn std::error::Error>> {
    Ok(match db.get(key)? {
        Some(x) => Some(deserialize(&x)?),
        None => None
    })
}

fn set<E: Serialize>(db: &Db, key: &str, value: &E) -> Result<(), Box<dyn std::error::Error>> {
    db.set(key, serialize(value)?)?;
    Ok(())
}

#[get("/")]
fn index(base: State<Db>) -> Result<String, Box<dyn std::error::Error>> {
    let mut visits = get::<usize>(&base, "visits")?.unwrap_or(0);
    visits += 1;
    set(&base, "visits", &visits)?;
    Ok(format!("Hello! You are visitor number {}", visits))
}

fn main() {
    let database = Db::start_default("database").unwrap();
    rocket::ignite()
        .mount("/", routes![index])
        .manage(database)
        .launch();
    print!("ENDING");
}
