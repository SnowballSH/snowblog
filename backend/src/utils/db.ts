import { DB } from "https://deno.land/x/sqlite/mod.ts";

// Define types for database operations
export type SqliteValue = string | number | boolean | null | Uint8Array;
export type Row = SqliteValue[];
export type QueryResult = Row[];

let db = new DB("blog.db");

export function useTestDb() {
  db = new DB();
  initDb();
}

export function initDb() {
  // Create tables if they don't exist
  db.execute(`
  CREATE TABLE IF NOT EXISTS blogs (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    createdAt DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
  );

  CREATE TABLE IF NOT EXISTS blog_tags (
    blogId TEXT,
    tagId TEXT,
    PRIMARY KEY (blogId, tagId),
    FOREIGN KEY (blogId) REFERENCES blogs(id),
    FOREIGN KEY (tagId) REFERENCES tags(id)
  );
`);
}

/**
 * Enhanced query method that returns properly typed results
 */
function query(sql: string, params: SqliteValue[] = []): QueryResult {
  return db.query(sql, params) as QueryResult;
}

export { db, query };