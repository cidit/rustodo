-- Add migration script here

CREATE TABLE IF NOT EXISTS todos (
    id    TEXT       PRIMARY KEY     NOT NULL,
    text  TEXT       NOT NULL,
    done  BOOLEAN    NOT NULL,
    date  DATETIME   NOT NULL
);
