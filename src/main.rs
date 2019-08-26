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
    Debt {
        owee: String,
        ower: String,
        amount: String,
    },
}

#[derive(Debug, FromForm)]
struct LoginData {
    username: String,
    password: String,
}

#[derive(Debug, FromForm)]
struct AddDebtData {
    user: String,
    owes: String,
    amount: String,
}

const USER_COOKIE_NAME: &'static str = "USER";

fn get<E: DeserializeOwned>(db: &Db, key: &str) -> Result<Option<E>, Box<dyn std::error::Error>> {
    Ok(match db.get(key.as_bytes())? {
        Some(x) => Some(deserialize(&x)?),
        None => None,
    })
}

fn set<E: Serialize>(db: &Db, key: &str, value: &E) -> Result<(), Box<dyn std::error::Error>> {
    db.set(key.as_bytes(), serialize(value)?)?;
    Ok(())
}

fn range<'a, E: DeserializeOwned>(
    db: &'a Db,
    start: &str,
    end: &str,
) -> impl Iterator<Item = Result<(String, E), Box<dyn std::error::Error + 'a>>> {
    db.range(start.as_bytes()..end.as_bytes()).map(
        |x| -> Result<(String, E), Box<dyn std::error::Error + 'a>> {
            let (k, v) = x?;
            Ok((String::from_utf8(k)?, deserialize(&v)?))
        },
    )
}

#[get("/static/<file..>")]
fn files(file: PathBuf) -> Result<NamedFile, status::NotFound<()>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|_| status::NotFound(()))
}

#[get("/")]
fn index(base: State<Db>, cookies: Cookies) -> Result<Template, Box<dyn std::error::Error>> {
    let mut visits = get::<usize>(&base, "Visits/HELLO")?.unwrap_or(0);
    visits += 1;
    #[derive(Serialize)]
    struct User {
        username: String,
        balance: String,
    }
    #[derive(Serialize)]
    struct TestContext {
        count: usize,
        current_user: String,
        users: Vec<User>,
    }
    set(&base, "Visits/HELLO", &visits)?;
    Ok(Template::render(
        "main",
        &TestContext {
            count: visits,
            current_user: get_current_user(cookies).unwrap_or("None".to_string()),
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

#[post("/add-debt", data = "<add_debt_data>")]
fn add_debt(
    mut cookies: Cookies,
    base: State<Db>,
    add_debt_data: Form<AddDebtData>,
) -> Result<Redirect, Box<dyn std::error::Error>> {
    println!("Saving {:?}", add_debt_data);
    set(
        &base,
        &DbKey::Debt {
            owee: add_debt_data.user.clone(),
            ower: "ower".to_string(),
            amount: add_debt_data.amount.clone(),
        },
        &hash("pass", DEFAULT_COST).unwrap(),
    )
    .unwrap();

    Ok(Redirect::to(uri!(index)))
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
    let original_password: String = match get(&base, &format!("userpass/{}", login_data.username)) {
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

fn get_current_user(mut cookies: Cookies) -> Option<String> {
    match &cookies.get_private(USER_COOKIE_NAME) {
        Some(c) => Some(c.value().to_string()),
        _ => None,
    }
}

fn main() {
    let database = Db::start_default("database").unwrap();
    set(
        &database,
        &"userpass/ben",
        &hash("pass", DEFAULT_COST).unwrap(),
    )
    .unwrap();

    rocket::ignite()
        .mount(
            "/",
            routes![
                index,
                files,
                login_form,
                login,
                logout,
                current_user,
                add_debt
            ],
        )
        .manage(database)
        .attach(Template::fairing())
        .launch();
    print!("ENDING");
}
