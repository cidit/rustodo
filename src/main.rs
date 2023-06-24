use futures_util::stream::TryStreamExt;
use std::fmt::Display;

use chrono::prelude::*;
use clap::{self, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use uuid::Uuid;

static _MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    subcommand: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    New {
        text: String,
    },
    List,
    Complete {
        id: i64,
        completed: Option<bool>,
    },
    Search {
        search_string: String,
        max_nb_entries: Option<usize>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TodoRecord {
    date: DateTime<Utc>,
    done: bool,
    id: Uuid,
    text: String,
}

struct Todos(Vec<TodoRecord>);

struct DisplayableTodoList {
    todos: Vec<TodoRecord>,
    /// in number of characters. truncates past this lenght.
    max_column_length: Option<usize>,
    /// in number of characters.
    /// overrides max_column_length for the `text` field if set.
    truncate_text_at: Option<usize>,
}

impl Display for DisplayableTodoList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn field_length(s: &[String]) -> Option<usize> {
            s.iter().map(|s| s.len()).max()
        }
        fn tmpname(todos: &[TodoRecord]) -> Option<()> {
            let id_cl = field_length(todos.iter().map(|t| t.id.to_string()).collect::<Vec<_>>().as_slice())?;
            Some(())
        }
        let id_column_length = self
            .todos
            .iter()
            .map(|t| &t.id)
            .map(|id| id.to_string())
            .map(|s| s.len())
            .max();
        let text_column_length = self
            .todos
            .iter()
            .map(|t| &t.text)
            .map(|s|s.len())
            .max()
            .min(self.truncate_text_at);
        unimplemented!()
    }
}

impl Display for Todos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const FIELD_LENGTH: usize = 8;
        fn field_fmt(str: &str) -> String {
            match str.len() {
                FIELD_LENGTH => str.to_owned(),
                FIELD_LENGTH.. => {
                    let mut str = str.to_owned();
                    str.truncate(FIELD_LENGTH - 3);
                    format!("{str}...")
                }
                _ => format!("{str: >FIELD_LENGTH$}"),
            }
        }

        fn todo_fmt(todo: &TodoRecord) -> String {
            let id = field_fmt(&todo.id.to_string());
            let text = field_fmt(&todo.text);
            let done = todo.done.then_some("X").unwrap_or(" ");
            let date = field_fmt(&todo.date.to_string());
            format!("|{id} {text}  [{done}]   {date}|")
        }

        write!(
            f,
            "
|{id: ^FIELD_LENGTH$}|{text: ^FIELD_LENGTH$}|{done: ^6}|{date: ^FIELD_LENGTH$}|
|{s}|{s}|------|{s}|
            ",
            id = "id",
            text = "text",
            done = "done",
            date = "date",
            s = "-".repeat(8)
        )?;
        for todo in &self.0 {
            writeln!(f, "{}", todo_fmt(&todo))?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    let db = sqlx::SqlitePool::connect(
        &std::env::var("DATABASE_URL").expect("env variable `DATABASE_URL` not set."),
    )
    .await?;

    use Command::*;
    match cli.subcommand {
        New { text } => {
            let uuid = Uuid::new_v4();
            let time = Utc::now();
            sqlx::query!(
                r#"
                INSERT INTO todos (id, text, done, date)
                VALUES (?, ?, ?, ?)
                "#,
                uuid,
                text,
                false,
                time,
            )
            .execute(&db)
            .await?;
        }
        List => {
            let todos: Vec<TodoRecord> = sqlx::query_as!(
                TodoRecord,
                r#"
                SELECT  id as "id: Uuid", 
                        text, 
                        done, 
                        date as "date: DateTime<Utc>"
                FROM todos
                "#
            )
            .fetch_all(&db)
            .await?;
            println!("{}", Todos(todos));
        }
        Complete { id, completed } => {
            let completedness = completed.unwrap_or(false);

            sqlx::query!(
                r#"
                UPDATE todos
                SET done = ?
                WHERE id = ?
                "#,
                completedness,
                id,
            )
            .execute(&db)
            .await?;
        }
        Search {
            search_string,
            max_nb_entries,
        } => {
            let mut results = Vec::new();
            let mut db_cursor = sqlx::query_as!(
                TodoRecord,
                r#"
                SELECT  id as "id: Uuid", 
                        done, 
                        text, 
                        date as "date: DateTime<Utc>" 
                FROM todos
                "#,
            )
            .fetch(&db);

            while let Some(todo) = db_cursor.try_next().await? {
                if format!("{todo:?}").contains(&search_string) {
                    results.push(todo);
                }
                // if there is a max number of entries, stop at that number, else continue
                // until all entries have been scanned
                // if let Some(nb) = max_nb_entries && nb == results.len() { UNSTABLE LANGUAGE FEATURE
                if max_nb_entries.map(|n| n == results.len()).unwrap_or(false) {
                    break;
                }
            }

            println!("{}", Todos(results));
        }
    };
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
}
