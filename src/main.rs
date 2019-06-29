#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::State;
use rocket::response::{NamedFile, status};
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use sled::Db;
use bincode::{serialize, deserialize};
use rocket_contrib::templates::Template;

#[derive(Serialize, Deserialize)]
enum DbKey {
    User(String)
}

fn get<E: DeserializeOwned>(db: &Db, key: &DbKey) -> Result<Option<E>, Box<dyn std::error::Error>> {
    Ok(match db.get(serialize(key)?)? {
        Some(x) => Some(deserialize(&x)?),
        None => None
    })
}

fn set<E: Serialize>(db: &Db, key: &DbKey, value: &E) -> Result<(), Box<dyn std::error::Error>> {
    db.set(serialize(key)?, serialize(value)?)?;
    Ok(())
}

#[get("/static/<file..>")]
fn files(file: PathBuf) -> Result<NamedFile, status::NotFound<()>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|_| status::NotFound(()))
}

#[get("/")]
fn index(base: State<Db>) -> Result<Template, Box<dyn std::error::Error>> {
    let mut visits = get::<usize>(&base, &DbKey::User("HELLO".into()))?.unwrap_or(0);
    visits += 1;
    #[derive(Serialize)]
    struct User {
        name: String,
        balance: f32,
    }
    #[derive(Serialize)]
    struct TestContext {
        count: usize,
        users: Vec<User>
    }
    set(&base, &DbKey::User("HELLO".into()), &visits)?;
    Ok(Template::render("main", &TestContext{
        count: visits,
        users: vec![
            User { name: "Bert".into(), balance: 0.5 }
        ]
    }))
}

fn main() {
    let database = Db::start_default("database").unwrap();
    rocket::ignite()
        .mount("/", routes![index, files])
        .manage(database)
        .attach(Template::fairing())
        .launch();
    print!("ENDING");
}
