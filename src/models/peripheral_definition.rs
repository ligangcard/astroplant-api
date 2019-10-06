use crate::schema::peripheral_definitions;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::{Identifiable, QueryResult, Queryable};

#[derive(Clone, Debug, PartialEq, Queryable, Identifiable)]
#[table_name = "peripheral_definitions"]
pub struct PeripheralDefinition {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub module_name: String,
    pub class_name: String,
    pub configuration_schema: serde_json::Value,
}

impl PeripheralDefinition {
    pub fn by_id(conn: &PgConnection, id: i32) -> QueryResult<Self> {
        peripheral_definitions::table.find(id).first(conn)
    }

    pub fn all(conn: &PgConnection) -> QueryResult<Vec<Self>> {
        peripheral_definitions::table.load(conn)
    }

    pub fn cursor_page(
        conn: &PgConnection,
        after: Option<i32>,
        limit: i64,
    ) -> QueryResult<Vec<Self>> {
        let q = peripheral_definitions::table.order(peripheral_definitions::columns::id.asc()).limit(limit);
        if let Some(after) = after {
            q.filter(peripheral_definitions::columns::id.gt(after)).load(conn)
        } else {
            q.load(conn)
        }
    }
}