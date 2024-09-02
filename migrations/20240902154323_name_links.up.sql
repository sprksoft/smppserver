-- Add up migration script here
CREATE TABLE name_links (
  name VARCHAR NOT NULL PRIMARY KEY,
  owner UUID NOT NULL,
  created_at INTEGER
);
