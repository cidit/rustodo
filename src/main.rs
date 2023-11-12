use futures_util::stream::TryStreamExt;
use std::fmt::Display;

use chrono::prelude::*;
use clap::{self, Parser, Subcommand};
use prettytable::Table;
use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use uuid::Uuid;

use rustodo::gui;
static _MIGRATOR: Migrator = sqlx::migrate!();


#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    subcommand: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    New {
        /// if there is no string, open an editor?
        text: Option<String>,
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
    Visual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TodoRecord {
    date: DateTime<Utc>,
    done: bool,
    id: Uuid,
    text: String,
}

struct DisplayTable {
    todos: Vec<TodoRecord>,
    // / in number of characters. truncates past this lenght.
    // max_column_length: Option<usize>,
    // / in number of characters.
    // / overrides max_column_length for the `text` field if set.
    // truncate_text_at: Option<usize>,
}

impl Display for DisplayTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(prettytable::row!["id", "text", "done", "date"]);
        for TodoRecord {
            id,
            text,
            done,
            date,
        } in &self.todos
        {
            table.add_row(prettytable::row![
                id.to_string(),
                text.chars()
                    .take(13)
                    .chain("...".chars())
                    .collect::<String>(),
                done.then_some("[X]").unwrap_or("[ ]"),
                date.to_string(),
            ]);
        }
        write!(f, "{table}")
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
            let text = text.unwrap_or_else(|| "editor!".to_owned() );
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
            println!("{}", DisplayTable { todos });
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

            println!("{}", DisplayTable { todos: results });
        }
        Visual => {
            gui::start(db)?;
        }
    };
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("GuiError: {0}")]
    GuiError(#[from] gui::GuiError),
}
