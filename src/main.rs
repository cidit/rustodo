use futures_util::stream::TryStreamExt;

use apply_if::ApplyIf;
use colored::Colorize;
use itertools::Itertools;
use std::fmt::Display;

use chrono::prelude::*;
use clap::{self, Parser, Subcommand};
use prettytable::Table;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustodo::gui;
static _MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

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
        id: Uuid,
        completed: Option<bool>,
    },
    Search {
        search_string: String,
        max_nb_entries: Option<usize>,
    },
    OpenView,
    /// this branch archived the old note by replacing it with a new note
    Edit {
        id: Uuid,
        text: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TodoRecord {
    date: DateTime<Utc>,
    done: bool,
    id: Uuid,
    text: String,
    replaced_by: Option<Uuid>,
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
        table.set_titles(prettytable::row!["id", "done", "text", "date", "archived"]);
        for TodoRecord {
            id,
            text,
            done,
            date,
            replaced_by,
        } in self
            .todos
            .iter()
            .sorted_by_key(|&tr| tr.date)
            .sorted_by_key(|&tr| tr.replaced_by.clone())
        // .sort_by(|s, o| {std::cmp::Ordering::Less})
        {
            table.add_row(prettytable::row![
                id.to_string(),
                // .chars()
                // .take(8)
                // .collect::<String>()
                // .on_green()
                // .bold()
                // .apply_if(archived != &Archival::No, |s| s.strikethrough()),
                if *done { "[X]" } else { "[ ]" },
                text.chars()
                    .take(32)
                    // .chain("...".chars())
                    .collect::<String>(),
                // .strikethrough(),
                date.date_naive().to_string(),
                replaced_by.map_or("No".to_owned(), |uuid| uuid.to_string())
            ]);
        }
        write!(f, "{table}")
    }
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    let db_client = sqlx::SqlitePool::connect(
        &std::env::var("DATABASE_URL").expect("env variable `DATABASE_URL` not set."),
    )
    .await?;
    use Command::*;
    match cli.subcommand {
        New { text } => {
            let uuid = Uuid::new_v4();
            let time = Utc::now();
            let text = text.unwrap_or_else(|| "editor!".to_owned());
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
            .execute(&db_client)
            .await?;
        }
        List => {
            let todos: Vec<TodoRecord> = sqlx::query_as!(
                TodoRecord,
                r#"
                SELECT  id as "id: Uuid", 
                        text, 
                        done, 
                        date as "date: DateTime<Utc>",
                        replaced_by as "replaced_by: Uuid"
                FROM todos
                "#
            )
            .fetch_all(&db_client)
            .await?;
            println!("{}", DisplayTable { todos });
        }
        Complete { id, completed } => {
            let completedness = completed.unwrap_or(true);
            // let condition = format!("%{id}%");
            sqlx::query!(
                r#"
                UPDATE todos
                SET done = ?
                WHERE id = ?
                "#,
                completedness,
                // condition,
                id // Uuid::from_str(id.),
            )
            .execute(&db_client)
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
                        date as "date: DateTime<Utc>",
                        replaced_by as "replaced_by: Uuid"
                FROM todos
                "#,
            )
            .fetch(&db_client);

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
        OpenView => {
            gui::start(db_client)?;
        }
        Edit { id, text } => {}
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
