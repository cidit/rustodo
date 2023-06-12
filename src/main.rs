use std::fmt::Display;

use serde::{Serialize, Deserialize};
use clap::{self, Parser, Subcommand};
use sqlx::{self, FromRow};
use chrono::prelude::*;
use uuid::Uuid;
use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    subcommand: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    New { text: String },
    List,
    Complete { id: i64, completed: Option<bool> },
    Search { search_string: String },
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
struct TodoRecord {
    date: DateTime<Utc>,
    done: bool,
    id: Uuid,
    text: String,
}

impl TodoRecord {
    pub fn id(&self) -> Uuid { self.id }
    pub fn text(&self) -> &str { &self.text }
    pub fn done(&self) -> bool { self.done }
    // pub fn done(&self) -> bool { self.done % 2 == 1}
    pub fn date(&self) -> DateTime<Utc> { self.date }

}

struct Todos(Vec<TodoRecord>);

impl Display for Todos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const FIELD_LENGTH: usize = 8;
        fn field_fmt(str: &str) -> String {
            match str.len() {
                FIELD_LENGTH => str.to_owned(),
                FIELD_LENGTH.. => {
                    let mut str = str.to_owned();
                    str.truncate(FIELD_LENGTH-3);
                    format!("{str}...")
                }
                _ => format!("{str: >FIELD_LENGTH$}"),
            }
        }

        fn todo_fmt(todo: &TodoRecord) -> String {
            let id = field_fmt(&todo.id().to_string());
            let text = field_fmt(&todo.text());
            let done = todo.done().then_some("X").unwrap_or(" ");
            let date = field_fmt(&todo.date().to_string());
            format!("|{id} {text} [{done}] {date}|")
        }

        writeln!(f, "
|{id:-^FIELD_LENGTH$}|{text:-^FIELD_LENGTH$}|{done:-^FIELD_LENGTH$}|{date:-^FIELD_LENGTH$}|
|{s}|{s}|---|{s}|
            ",
            id="id",
            text="text",
            done="done",
            date="date",
            s="s".repeat(8)
        )?;
        for todo in &self.0 {
            writeln!(f, "{}", todo_fmt(&todo))?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let db = sqlx::SqlitePool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    use Command::*;
    match cli.subcommand {
        New { text } => {
            todo!()
        },
        List => {
            let todos: Vec<TodoRecord> = sqlx::query_as!(
                TodoRecord,
                r#"
SELECT id, text, done, date 
FROM todos
        "#
            )
            .fetch_all(&db)
            .await
            .unwrap();
            for todo in todos {
                
            }
        }
        Complete { id, completed } => todo!(),
        Search { search_string } => todo!(),
    };
}
