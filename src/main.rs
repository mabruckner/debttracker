#![feature(proc_macro_hygiene, decl_macro)]

extern crate bcrypt;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate failure;

use bcrypt::{hash, verify, DEFAULT_COST};
use bincode::{deserialize, serialize};
use rocket::http::{Cookie, Cookies};
use rocket::request::Form;
use rocket::response::{status, NamedFile, Redirect};
use rocket::State;
use rocket_contrib::templates::Template;
use rusqlite::types::ToSql;
use rusqlite::{params, Connection, Error};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

mod money;
use money::*;

type DbConn = Mutex<Connection>;

#[derive(Debug, Serialize, Deserialize)]
struct Debt {
  creditor: String,
  debtor: String,
  time: SystemTime,
  amount: Money,
}

impl Debt {
  /// Swap owee/ower and negate amount
  fn clone_negated(&self) -> Debt {
    Debt {
      creditor: self.debtor.clone(),
      debtor: self.creditor.clone(),
      time: self.time.clone(),
      amount: -self.amount.clone(),
    }
  }
}

#[derive(Debug, FromForm)]
struct LoginData {
  username: String,
  password: String,
}

#[derive(Debug, FromForm)]
struct AddDebtData {
  user: String,
  owe_direction: String,
  amount: String,
}

#[derive(Debug)]
struct User {
  username: String,
  password: String,
}

const USER_COOKIE_NAME: &'static str = "USER";

fn init_database(conn: &Connection) {
  conn
    .execute(
      "CREATE TABLE debts (
                  id              INTEGER PRIMARY KEY,
                  debtor          TEXT NOT NULL,
                  creditor        TEXT NOT NULL,
                  amount          TEXT NOT NULL,
                  time            TIME NOT NULL
                  )",
      &[] as &[&dyn ToSql],
    )
    .expect("create debts table");

  conn
    .execute(
      "CREATE TABLE users (
                  id              INTEGER PRIMARY KEY,
                  username        TEXT NOT NULL,
                  password        TEXT NOT NULL
                  )",
      &[] as &[&dyn ToSql],
    )
    .expect("create debts table");

  conn
    .execute(
      "INSERT INTO users (username, password) VALUES ($1, $2)",
      &[&"ben", hash("pass", DEFAULT_COST).unwrap().as_str()],
    )
    .unwrap();
  conn
    .execute(
      "INSERT INTO users (username, password) VALUES ($1, $2)",
      &[&"mitchell", hash("pass", DEFAULT_COST).unwrap().as_str()],
    )
    .unwrap();
}

#[get("/static/<file..>")]
fn files(file: PathBuf) -> Result<NamedFile, status::NotFound<()>> {
  let path = Path::new("static/").join(file);
  NamedFile::open(&path).map_err(|_| status::NotFound(()))
}

#[get("/")]
fn index(cookies: Cookies) -> Result<Template, Box<dyn std::error::Error>> {
  #[derive(Debug, Serialize)]
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
  Ok(Template::render(
    "main",
    &TestContext {
      count: 1,
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
  add_debt_data: Form<AddDebtData>,
) -> Result<Redirect, Box<dyn std::error::Error>> {
  println!("Saving {:?}", add_debt_data);
  let now = SystemTime::now();
  let nanos = now.duration_since(SystemTime::UNIX_EPOCH)?.as_nanos();

  let current_user = get_current_user(cookies).unwrap_or("None".to_string());
  let other_user = add_debt_data.user.clone();
  let owe_direction = add_debt_data.owe_direction.clone();

  let debtor;
  let creditor;
  if owe_direction == "owes" {
    creditor = current_user;
    debtor = other_user;
  } else {
    debtor = current_user;
    creditor = other_user;
  }

  let debt = Debt {
    creditor: creditor.clone(),
    debtor: debtor.clone(),
    amount: Money::from_money_string(add_debt_data.amount.clone())?,
    time: now,
  };

  // set(&base, &format!("debts/{}/{}", creditor, nanos), &debt).unwrap();
  // set(
  //   &base,
  //   &format!("debts/{}/{}", debtor, nanos),
  //   &debt.clone_negated(),
  // )
  // .unwrap();

  Ok(Redirect::to(uri!(index)))
}

#[get("/login")]
fn login_form() -> Result<Template, Box<dyn std::error::Error>> {
  let context: HashMap<&str, &str> = HashMap::new();
  Ok(Template::render("login", context))
}

#[post("/login", data = "<login_data>")]
fn login(
  db_conn: State<'_, DbConn>,
  mut cookies: Cookies,
  login_data: Form<LoginData>,
) -> Result<String, Box<dyn std::error::Error>> {
  // curl -v -X POST -d 'username=ben&password=pass' http://localhost:8000/login -H "Content-Type: application/x-www-form-urlencoded"
  let session = db_conn.lock().unwrap();
  let mut stmt = session.prepare("SELECT username, password FROM users WHERE username='ben'")?;

  let user = stmt
    .query_map(params![], |row| {
      Ok(User {
        username: row.get(0)?,
        password: row.get(1)?,
      })
    })?
    .next();

  if let Some(user) = user {
    let is_password_correct = verify(&login_data.password, &user?.password);

    if is_password_correct.unwrap_or(false) {
      cookies.add_private(Cookie::new(USER_COOKIE_NAME, login_data.username.clone()));
      Ok(format!("Good"))
    } else {
      Ok(format!("Bad"))
    }
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
  let conn = Connection::open_in_memory().expect("in memory db");

  init_database(&conn);

  rocket::ignite()
    .manage(Mutex::new(conn))
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
    .attach(Template::fairing())
    .launch();
  print!("ENDING");
}
