-- Add migration script here

CREATE TABLE IF NOT EXISTS todos (
    id    BLOB       PRIMARY KEY,
    text  TEXT       NOT NULL,
    done  BOOLEAN    NOT NULL,
    date  DATETIME   NOT NULL
);
