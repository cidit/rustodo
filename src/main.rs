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
}

#[derive(Ord, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
enum Archival {
    Yes,
    No,
    Replaced(Uuid),
}

impl Display for Archival {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("couldn't deserialize")]
struct DeserializationErr;

impl std::str::FromStr for Archival {
    type Err = DeserializationErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Yes" => Ok(Self::Yes),
            "No" => Ok(Self::No),
            other => {
                if other.starts_with("Replaced") {
                    let rest = other.trim_start_matches("Replaced");
                    let uuid = &rest[1..rest.len() - 1];
                    let uuid = Uuid::from_str(uuid).map_err(|_| DeserializationErr)?;
                    return Ok(Self::Replaced(uuid));
                }
                return Err(DeserializationErr);
            }
        }
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for Archival
where
    &'r str: sqlx::Decode<'r, sqlx::Sqlite>,
{
    fn decode(
        value: <sqlx::Sqlite as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s = String::decode(value)?;
        Ok(s.parse()?)
    }
}

impl<'r> sqlx::Encode<'r, sqlx::Sqlite> for Archival {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::database::HasArguments<'r>>::ArgumentBuffer, //: &mut <DB as sqlx::database::HasArguments<'r>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        let displayed = self.to_string();
        displayed.encode(buf)
    }
}

impl PartialOrd for Archival {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let priority = |arch: &Self| match arch {
            Self::Yes => 3,
            Self::Replaced(_) => 2,
            Self::No => 1,
        };

        Some(priority(self).cmp(&priority(other)))
    }
}

impl sqlx::Type<sqlx::Sqlite> for Archival {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        String::type_info()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TodoRecord {
    date: DateTime<Utc>,
    done: bool,
    id: Uuid,
    text: String,
    archived: Option<Uuid>,
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
            archived,
        } in self
            .todos
            .iter()
            .sorted_by_key(|&tr| tr.date)
            .sorted_by_key(|&tr| tr.archived.clone())
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
                archived.map_or("No".to_owned(), |uuid| uuid.to_string())
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
                INSERT INTO todos (id, text, done, date, archived)
                VALUES (?, ?, ?, ?, ?)
                "#,
                uuid,
                text,
                false,
                time,
                Archival::No,
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
                        archived as "archived: Archival"
                FROM todos
                "#
            )
            .fetch_all(&db_client)
            .await?;
            println!("{}", DisplayTable { todos });
        }
        Complete { id, completed } => {
            let completedness = completed.unwrap_or(false);
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
                        archived as "archived: Archival"
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
