#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate bcrypt;

use bcrypt::{hash, verify, DEFAULT_COST};
use bincode::{deserialize, serialize};
use rocket::http::{Cookie, Cookies};
use rocket::request::Form;
use rocket::response::{status, NamedFile, Redirect};
use rocket::State;
use rocket_contrib::templates::Template;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod money;
use money::*;

#[derive(Serialize, Deserialize)]
enum DbKey {
    UsernameToPassword(String),
    Visits(String),
}

#[derive(FromForm)]
struct LoginData {
    username: String,
    password: String,
}

const USER_COOKIE_NAME: &'static str = "USER";

fn get<E: DeserializeOwned>(db: &Db, key: &DbKey) -> Result<Option<E>, Box<dyn std::error::Error>> {
    Ok(match db.get(serialize(key)?)? {
        Some(x) => Some(deserialize(&x)?),
        None => None,
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
    let mut visits = get::<usize>(&base, &DbKey::Visits("HELLO".into()))?.unwrap_or(0);
    visits += 1;
    #[derive(Serialize)]
    struct User {
        username: String,
        balance: String,
    }
    #[derive(Serialize)]
    struct TestContext {
        count: usize,
        users: Vec<User>,
    }
    set(&base, &DbKey::Visits("HELLO".into()), &visits)?;
    Ok(Template::render(
        "main",
        &TestContext {
            count: visits,
            users: vec![
                User {
                    username: "bert".into(),
                    balance: Money::from_dollars(30).to_string(),
                },
                User {
                    username: "ben".into(),
                    balance: Money::from_dollars(20).to_string(),
                },
                User {
                    username: "mitchell".into(),
                    balance: Money::from_dollars(-20).to_string(),
                },
            ],
        },
    ))
}

#[get("/login")]
fn login_form() -> Result<Template, Box<dyn std::error::Error>> {
    let context: HashMap<&str, &str> = HashMap::new();
    Ok(Template::render("login", context))
}

#[post("/login", data = "<login_data>")]
fn login(
    mut cookies: Cookies,
    base: State<Db>,
    login_data: Form<LoginData>,
) -> Result<String, Box<dyn std::error::Error>> {
    // curl -v -X POST -d 'username=ben&password=pass' http://localhost:8000/login -H "Content-Type: application/x-www-form-urlencoded"
    let original_password: String = match get(
        &base,
        &DbKey::UsernameToPassword(login_data.username.clone()),
    ) {
        Ok(Some(password)) => password,
        _ => return Ok(format!("Bad")),
    };

    let is_password_correct = verify(&login_data.password, &original_password);

    if is_password_correct.unwrap_or(false) {
        cookies.add_private(Cookie::new(USER_COOKIE_NAME, login_data.username.clone()));
        Ok(format!("Good"))
    } else {
        Ok(format!("Bad"))
    }
}

#[get("/logout")]
fn logout(mut cookies: Cookies) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(existing_cookie) = cookies.get_private(USER_COOKIE_NAME) {
        cookies.remove_private(existing_cookie);
    }

    Ok(format!("Good"))
}

#[get("/current-user")]
fn current_user(mut cookies: Cookies) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "User: {}",
        match &cookies.get_private(USER_COOKIE_NAME) {
            Some(c) => c.value(),
            _ => "None",
        }
    ))
}

fn check_auth(mut cookies: Cookies) -> Option<Redirect> {
    if let None = cookies.get_private(USER_COOKIE_NAME) {
        return Some(Redirect::to(uri!(login)));
    }
    return None;
}

fn main() {
    let database = Db::start_default("database").unwrap();
    set(
        &database,
        &DbKey::UsernameToPassword("ben".to_string()),
        &hash("pass", DEFAULT_COST).unwrap(),
    )
    .unwrap();

    rocket::ignite()
        .mount(
            "/",
            routes![index, files, login_form, login, logout, current_user],
        )
        .manage(database)
        .attach(Template::fairing())
        .launch();
    print!("ENDING");
}
