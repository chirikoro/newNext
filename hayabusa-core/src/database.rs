//! Database ORM / Query Builder for Hayabusa.
//!
//! Type-safe query builder for SQL databases (SQLite, Postgres, MySQL).
//! Supports migrations, connection pooling config, and transactions.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let query = Query::select("users")
//!     .columns(&["id", "name", "email"])
//!     .where_eq("active", "true")
//!     .order_by("created_at", Order::Desc)
//!     .limit(10);
//! println!("{}", query.to_sql());
//! ```

use std::collections::HashMap;

// ─── Query Builder ──────────────────────────────────────────

/// SQL query builder
#[derive(Debug, Clone)]
pub struct Query {
    pub kind: QueryKind,
    pub table: String,
    pub columns: Vec<String>,
    pub conditions: Vec<Condition>,
    pub order: Vec<(String, Order)>,
    pub limit_val: Option<u64>,
    pub offset_val: Option<u64>,
    pub values: Vec<(String, SqlValue)>,
    pub joins: Vec<Join>,
    pub group_by: Vec<String>,
    pub having: Vec<Condition>,
    pub returning: Vec<String>,
    pub distinct: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryKind {
    Select,
    Insert,
    Update,
    Delete,
    Upsert,
}

#[derive(Debug, Clone)]
pub enum SqlValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Null,
    Param(String), // named parameter placeholder
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub field: String,
    pub op: CondOp,
    pub value: SqlValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CondOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    ILike,
    In,
    IsNull,
    IsNotNull,
    Between,
}

#[derive(Debug, Clone)]
pub struct Join {
    pub kind: JoinKind,
    pub table: String,
    pub on: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
}

impl Query {
    // ── Constructors ──

    pub fn select(table: &str) -> Self {
        Self::new(QueryKind::Select, table)
    }

    pub fn insert(table: &str) -> Self {
        Self::new(QueryKind::Insert, table)
    }

    pub fn update(table: &str) -> Self {
        Self::new(QueryKind::Update, table)
    }

    pub fn delete(table: &str) -> Self {
        Self::new(QueryKind::Delete, table)
    }

    fn new(kind: QueryKind, table: &str) -> Self {
        Self {
            kind,
            table: table.to_string(),
            columns: Vec::new(),
            conditions: Vec::new(),
            order: Vec::new(),
            limit_val: None,
            offset_val: None,
            values: Vec::new(),
            joins: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            returning: Vec::new(),
            distinct: false,
        }
    }

    // ── SELECT modifiers ──

    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.columns = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn column(mut self, col: &str) -> Self {
        self.columns.push(col.to_string());
        self
    }

    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    // ── WHERE conditions ──

    pub fn where_eq(mut self, field: &str, value: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::Eq, value: SqlValue::Text(value.into()) });
        self
    }

    pub fn where_neq(mut self, field: &str, value: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::Neq, value: SqlValue::Text(value.into()) });
        self
    }

    pub fn where_gt(mut self, field: &str, value: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::Gt, value: SqlValue::Text(value.into()) });
        self
    }

    pub fn where_gte(mut self, field: &str, value: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::Gte, value: SqlValue::Text(value.into()) });
        self
    }

    pub fn where_lt(mut self, field: &str, value: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::Lt, value: SqlValue::Text(value.into()) });
        self
    }

    pub fn where_like(mut self, field: &str, pattern: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::Like, value: SqlValue::Text(pattern.into()) });
        self
    }

    pub fn where_null(mut self, field: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::IsNull, value: SqlValue::Null });
        self
    }

    pub fn where_not_null(mut self, field: &str) -> Self {
        self.conditions.push(Condition { field: field.into(), op: CondOp::IsNotNull, value: SqlValue::Null });
        self
    }

    pub fn where_in(mut self, field: &str, values: &[&str]) -> Self {
        let list = values.iter().map(|v| format!("'{}'", sql_escape(v))).collect::<Vec<_>>().join(", ");
        self.conditions.push(Condition { field: field.into(), op: CondOp::In, value: SqlValue::Text(list) });
        self
    }

    // ── ORDER / LIMIT / OFFSET ──

    pub fn order_by(mut self, field: &str, order: Order) -> Self {
        self.order.push((field.to_string(), order));
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.limit_val = Some(n);
        self
    }

    pub fn offset(mut self, n: u64) -> Self {
        self.offset_val = Some(n);
        self
    }

    // ── JOIN ──

    pub fn join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(Join { kind: JoinKind::Inner, table: table.into(), on: on.into() });
        self
    }

    pub fn left_join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(Join { kind: JoinKind::Left, table: table.into(), on: on.into() });
        self
    }

    // ── GROUP BY / HAVING ──

    pub fn group_by(mut self, field: &str) -> Self {
        self.group_by.push(field.to_string());
        self
    }

    // ── INSERT/UPDATE values ──

    pub fn set(mut self, field: &str, value: SqlValue) -> Self {
        self.values.push((field.to_string(), value));
        self
    }

    pub fn set_str(mut self, field: &str, value: &str) -> Self {
        self.values.push((field.to_string(), SqlValue::Text(value.to_string())));
        self
    }

    pub fn set_int(mut self, field: &str, value: i64) -> Self {
        self.values.push((field.to_string(), SqlValue::Integer(value)));
        self
    }

    pub fn set_bool(mut self, field: &str, value: bool) -> Self {
        self.values.push((field.to_string(), SqlValue::Bool(value)));
        self
    }

    pub fn set_null(mut self, field: &str) -> Self {
        self.values.push((field.to_string(), SqlValue::Null));
        self
    }

    // ── RETURNING ──

    pub fn returning(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    // ── SQL Generation ──

    pub fn to_sql(&self) -> String {
        match self.kind {
            QueryKind::Select => self.build_select(),
            QueryKind::Insert => self.build_insert(),
            QueryKind::Update => self.build_update(),
            QueryKind::Delete => self.build_delete(),
            QueryKind::Upsert => self.build_insert(), // ON CONFLICT handled separately
        }
    }

    fn build_select(&self) -> String {
        let distinct = if self.distinct { "DISTINCT " } else { "" };
        let cols = if self.columns.is_empty() { "*".to_string() } else { self.columns.join(", ") };
        let mut sql = format!("SELECT {}{} FROM {}", distinct, cols, self.table);

        for join in &self.joins {
            let kind = match join.kind {
                JoinKind::Inner => "INNER JOIN",
                JoinKind::Left => "LEFT JOIN",
                JoinKind::Right => "RIGHT JOIN",
                JoinKind::Full => "FULL OUTER JOIN",
            };
            sql.push_str(&format!(" {} {} ON {}", kind, join.table, join.on));
        }

        self.append_where(&mut sql);

        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        self.append_order(&mut sql);
        self.append_limit(&mut sql);
        sql
    }

    fn build_insert(&self) -> String {
        let fields: Vec<&str> = self.values.iter().map(|(f, _)| f.as_str()).collect();
        let vals: Vec<String> = self.values.iter().map(|(_, v)| sql_value_str(v)).collect();
        let mut sql = format!("INSERT INTO {} ({}) VALUES ({})", self.table, fields.join(", "), vals.join(", "));
        self.append_returning(&mut sql);
        sql
    }

    fn build_update(&self) -> String {
        let sets: Vec<String> = self.values.iter().map(|(f, v)| format!("{} = {}", f, sql_value_str(v))).collect();
        let mut sql = format!("UPDATE {} SET {}", self.table, sets.join(", "));
        self.append_where(&mut sql);
        self.append_returning(&mut sql);
        sql
    }

    fn build_delete(&self) -> String {
        let mut sql = format!("DELETE FROM {}", self.table);
        self.append_where(&mut sql);
        self.append_returning(&mut sql);
        sql
    }

    fn append_where(&self, sql: &mut String) {
        if !self.conditions.is_empty() {
            let clauses: Vec<String> = self.conditions.iter().map(|c| {
                match c.op {
                    CondOp::Eq => format!("{} = {}", c.field, sql_value_str(&c.value)),
                    CondOp::Neq => format!("{} != {}", c.field, sql_value_str(&c.value)),
                    CondOp::Gt => format!("{} > {}", c.field, sql_value_str(&c.value)),
                    CondOp::Gte => format!("{} >= {}", c.field, sql_value_str(&c.value)),
                    CondOp::Lt => format!("{} < {}", c.field, sql_value_str(&c.value)),
                    CondOp::Lte => format!("{} <= {}", c.field, sql_value_str(&c.value)),
                    CondOp::Like => format!("{} LIKE {}", c.field, sql_value_str(&c.value)),
                    CondOp::ILike => format!("{} ILIKE {}", c.field, sql_value_str(&c.value)),
                    CondOp::In => format!("{} IN ({})", c.field, if let SqlValue::Text(ref s) = c.value { s.clone() } else { String::new() }),
                    CondOp::IsNull => format!("{} IS NULL", c.field),
                    CondOp::IsNotNull => format!("{} IS NOT NULL", c.field),
                    CondOp::Between => format!("{} BETWEEN {}", c.field, sql_value_str(&c.value)),
                }
            }).collect();
            sql.push_str(&format!(" WHERE {}", clauses.join(" AND ")));
        }
    }

    fn append_order(&self, sql: &mut String) {
        if !self.order.is_empty() {
            let orders: Vec<String> = self.order.iter().map(|(f, o)| {
                format!("{} {}", f, match o { Order::Asc => "ASC", Order::Desc => "DESC" })
            }).collect();
            sql.push_str(&format!(" ORDER BY {}", orders.join(", ")));
        }
    }

    fn append_limit(&self, sql: &mut String) {
        if let Some(l) = self.limit_val {
            sql.push_str(&format!(" LIMIT {}", l));
        }
        if let Some(o) = self.offset_val {
            sql.push_str(&format!(" OFFSET {}", o));
        }
    }

    fn append_returning(&self, sql: &mut String) {
        if !self.returning.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning.join(", ")));
        }
    }
}

// ─── Migration ──────────────────────────────────────────────

/// Database migration
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: u32,
    pub name: String,
    pub up_sql: String,
    pub down_sql: String,
}

impl Migration {
    pub fn new(version: u32, name: &str, up: &str, down: &str) -> Self {
        Self { version, name: name.into(), up_sql: up.into(), down_sql: down.into() }
    }
}

/// Migration runner
#[derive(Debug, Clone)]
pub struct MigrationRunner {
    pub migrations: Vec<Migration>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self { migrations: Vec::new() }
    }

    pub fn add(mut self, migration: Migration) -> Self {
        self.migrations.push(migration);
        self.migrations.sort_by_key(|m| m.version);
        self
    }

    /// Generate the SQL to create the migrations tracking table
    pub fn init_sql() -> &'static str {
        "CREATE TABLE IF NOT EXISTS _hayabusa_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT DEFAULT CURRENT_TIMESTAMP)"
    }

    /// Generate SQL to run all pending migrations up to a version
    pub fn up_to(&self, target: u32) -> Vec<&Migration> {
        self.migrations.iter().filter(|m| m.version <= target).collect()
    }

    /// Get rollback SQL for a specific version
    pub fn down(&self, version: u32) -> Option<&str> {
        self.migrations.iter().find(|m| m.version == version).map(|m| m.down_sql.as_str())
    }
}

impl Default for MigrationRunner {
    fn default() -> Self { Self::new() }
}

// ─── Schema Builder ─────────────────────────────────────────

/// SQL table schema builder
#[derive(Debug, Clone)]
pub struct TableBuilder {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub if_not_exists: bool,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: String,
    pub primary_key: bool,
    pub not_null: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub references: Option<String>,
}

impl TableBuilder {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), columns: Vec::new(), if_not_exists: true }
    }

    pub fn id(mut self) -> Self {
        self.columns.push(ColumnDef {
            name: "id".into(), col_type: "INTEGER".into(),
            primary_key: true, not_null: true, unique: false, default: None, references: None,
        });
        self
    }

    pub fn uuid_id(mut self) -> Self {
        self.columns.push(ColumnDef {
            name: "id".into(), col_type: "TEXT".into(),
            primary_key: true, not_null: true, unique: false,
            default: Some("gen_random_uuid()".into()), references: None,
        });
        self
    }

    pub fn text(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef { name: name.into(), col_type: "TEXT".into(), primary_key: false, not_null: false, unique: false, default: None, references: None });
        self
    }

    pub fn integer(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef { name: name.into(), col_type: "INTEGER".into(), primary_key: false, not_null: false, unique: false, default: None, references: None });
        self
    }

    pub fn boolean(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef { name: name.into(), col_type: "BOOLEAN".into(), primary_key: false, not_null: false, unique: false, default: None, references: None });
        self
    }

    pub fn timestamp(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef { name: name.into(), col_type: "TIMESTAMP".into(), primary_key: false, not_null: false, unique: false, default: None, references: None });
        self
    }

    pub fn timestamps(self) -> Self {
        self.timestamp("created_at").timestamp("updated_at")
    }

    pub fn not_null(mut self) -> Self {
        if let Some(last) = self.columns.last_mut() { last.not_null = true; }
        self
    }

    pub fn unique(mut self) -> Self {
        if let Some(last) = self.columns.last_mut() { last.unique = true; }
        self
    }

    pub fn default(mut self, val: &str) -> Self {
        if let Some(last) = self.columns.last_mut() { last.default = Some(val.into()); }
        self
    }

    pub fn references(mut self, table_col: &str) -> Self {
        if let Some(last) = self.columns.last_mut() { last.references = Some(table_col.into()); }
        self
    }

    /// Generate CREATE TABLE SQL
    pub fn to_sql(&self) -> String {
        let exists = if self.if_not_exists { "IF NOT EXISTS " } else { "" };
        let cols: Vec<String> = self.columns.iter().map(|c| {
            let mut def = format!("{} {}", c.name, c.col_type);
            if c.primary_key { def.push_str(" PRIMARY KEY"); }
            if c.not_null { def.push_str(" NOT NULL"); }
            if c.unique { def.push_str(" UNIQUE"); }
            if let Some(ref d) = c.default { def.push_str(&format!(" DEFAULT {}", d)); }
            if let Some(ref r) = c.references { def.push_str(&format!(" REFERENCES {}", r)); }
            def
        }).collect();
        format!("CREATE TABLE {}{} (\n  {}\n)", exists, self.name, cols.join(",\n  "))
    }
}

// ─── Connection Config ──────────────────────────────────────

/// Database connection configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub driver: DbDriver,
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DbDriver {
    Sqlite,
    Postgres,
    Mysql,
}

impl DbConfig {
    pub fn sqlite(path: &str) -> Self {
        Self {
            driver: DbDriver::Sqlite, url: format!("sqlite://{}", path),
            max_connections: 5, min_connections: 1, connect_timeout_secs: 5, idle_timeout_secs: 300,
        }
    }

    pub fn postgres(url: &str) -> Self {
        Self {
            driver: DbDriver::Postgres, url: url.into(),
            max_connections: 20, min_connections: 2, connect_timeout_secs: 5, idle_timeout_secs: 300,
        }
    }

    pub fn mysql(url: &str) -> Self {
        Self {
            driver: DbDriver::Mysql, url: url.into(),
            max_connections: 20, min_connections: 2, connect_timeout_secs: 5, idle_timeout_secs: 300,
        }
    }

    pub fn from_env() -> Option<Self> {
        std::env::var("DATABASE_URL").ok().map(|url| {
            if url.starts_with("sqlite") { Self::sqlite(&url[9..]) }
            else if url.starts_with("postgres") { Self::postgres(&url) }
            else if url.starts_with("mysql") { Self::mysql(&url) }
            else { Self::sqlite(&url) }
        })
    }
}

// ─── Helpers ────────────────────────────────────────────────

fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn sql_value_str(v: &SqlValue) -> String {
    match v {
        SqlValue::Text(s) => format!("'{}'", sql_escape(s)),
        SqlValue::Integer(n) => n.to_string(),
        SqlValue::Float(n) => format!("{}", n),
        SqlValue::Bool(b) => if *b { "TRUE".into() } else { "FALSE".into() },
        SqlValue::Null => "NULL".into(),
        SqlValue::Param(name) => format!("${}", name),
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_basic() {
        let sql = Query::select("users").to_sql();
        assert_eq!(sql, "SELECT * FROM users");
    }

    #[test]
    fn test_select_columns() {
        let sql = Query::select("users").columns(&["id", "name"]).to_sql();
        assert_eq!(sql, "SELECT id, name FROM users");
    }

    #[test]
    fn test_select_where() {
        let sql = Query::select("users").where_eq("active", "true").to_sql();
        assert_eq!(sql, "SELECT * FROM users WHERE active = 'true'");
    }

    #[test]
    fn test_select_complex() {
        let sql = Query::select("posts")
            .columns(&["id", "title"])
            .where_eq("published", "true")
            .where_like("title", "%rust%")
            .order_by("created_at", Order::Desc)
            .limit(10)
            .offset(20)
            .to_sql();
        assert!(sql.contains("WHERE published = 'true' AND title LIKE '%rust%'"));
        assert!(sql.contains("ORDER BY created_at DESC"));
        assert!(sql.contains("LIMIT 10 OFFSET 20"));
    }

    #[test]
    fn test_select_join() {
        let sql = Query::select("posts")
            .left_join("users", "posts.author_id = users.id")
            .to_sql();
        assert!(sql.contains("LEFT JOIN users ON posts.author_id = users.id"));
    }

    #[test]
    fn test_select_distinct() {
        let sql = Query::select("tags").column("name").distinct().to_sql();
        assert!(sql.starts_with("SELECT DISTINCT"));
    }

    #[test]
    fn test_select_where_in() {
        let sql = Query::select("users").where_in("role", &["admin", "editor"]).to_sql();
        assert!(sql.contains("IN ('admin', 'editor')"));
    }

    #[test]
    fn test_select_where_null() {
        let sql = Query::select("users").where_null("deleted_at").to_sql();
        assert!(sql.contains("deleted_at IS NULL"));
    }

    #[test]
    fn test_insert() {
        let sql = Query::insert("users")
            .set_str("name", "Alice")
            .set_str("email", "alice@example.com")
            .set_bool("active", true)
            .to_sql();
        assert!(sql.contains("INSERT INTO users"));
        assert!(sql.contains("'Alice'"));
        assert!(sql.contains("TRUE"));
    }

    #[test]
    fn test_insert_returning() {
        let sql = Query::insert("users")
            .set_str("name", "Bob")
            .returning(&["id"])
            .to_sql();
        assert!(sql.contains("RETURNING id"));
    }

    #[test]
    fn test_update() {
        let sql = Query::update("users")
            .set_str("name", "Alice Updated")
            .where_eq("id", "1")
            .to_sql();
        assert!(sql.contains("UPDATE users SET name = 'Alice Updated'"));
        assert!(sql.contains("WHERE id = '1'"));
    }

    #[test]
    fn test_delete() {
        let sql = Query::delete("sessions")
            .where_lt("expires_at", "2025-01-01")
            .to_sql();
        assert!(sql.contains("DELETE FROM sessions"));
        assert!(sql.contains("WHERE expires_at < '2025-01-01'"));
    }

    #[test]
    fn test_sql_escape() {
        let sql = Query::insert("posts")
            .set_str("title", "It's a test")
            .to_sql();
        assert!(sql.contains("It''s a test"));
    }

    #[test]
    fn test_table_builder() {
        let sql = TableBuilder::new("users")
            .id()
            .text("name").not_null()
            .text("email").not_null().unique()
            .boolean("active").default("TRUE")
            .timestamps()
            .to_sql();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS users"));
        assert!(sql.contains("id INTEGER PRIMARY KEY"));
        assert!(sql.contains("name TEXT NOT NULL"));
        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
        assert!(sql.contains("active BOOLEAN DEFAULT TRUE"));
        assert!(sql.contains("created_at TIMESTAMP"));
    }

    #[test]
    fn test_table_references() {
        let sql = TableBuilder::new("posts")
            .id()
            .integer("author_id").not_null().references("users(id)")
            .to_sql();
        assert!(sql.contains("REFERENCES users(id)"));
    }

    #[test]
    fn test_migration() {
        let m = Migration::new(1, "create_users", "CREATE TABLE users (id INTEGER PRIMARY KEY)", "DROP TABLE users");
        assert_eq!(m.version, 1);
    }

    #[test]
    fn test_migration_runner() {
        let runner = MigrationRunner::new()
            .add(Migration::new(2, "add_posts", "CREATE TABLE posts ...", "DROP TABLE posts"))
            .add(Migration::new(1, "add_users", "CREATE TABLE users ...", "DROP TABLE users"));
        let up = runner.up_to(1);
        assert_eq!(up.len(), 1);
        assert_eq!(up[0].name, "add_users");
        assert!(runner.down(2).is_some());
    }

    #[test]
    fn test_db_config_sqlite() {
        let cfg = DbConfig::sqlite("data.db");
        assert_eq!(cfg.driver, DbDriver::Sqlite);
        assert!(cfg.url.contains("data.db"));
    }

    #[test]
    fn test_db_config_postgres() {
        let cfg = DbConfig::postgres("postgres://localhost/mydb");
        assert_eq!(cfg.driver, DbDriver::Postgres);
        assert_eq!(cfg.max_connections, 20);
    }

    #[test]
    fn test_group_by() {
        let sql = Query::select("orders")
            .columns(&["status", "COUNT(*)"])
            .group_by("status")
            .to_sql();
        assert!(sql.contains("GROUP BY status"));
    }

    #[test]
    fn test_set_null() {
        let sql = Query::update("users").set_null("avatar").where_eq("id", "1").to_sql();
        assert!(sql.contains("avatar = NULL"));
    }
}
