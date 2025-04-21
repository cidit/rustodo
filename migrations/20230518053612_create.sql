-- Add migration script here

CREATE TABLE IF NOT EXISTS todos (
    id    UUID       PRIMARY KEY     NOT NULL,
    text  TEXT       NOT NULL,
    done  BOOLEAN    NOT NULL,
    date  DATETIME   NOT NULL,
    archived UUID,
);
