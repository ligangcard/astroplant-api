use crate::schema::users;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::{Connection, Identifiable, QueryResult, Queryable};
use validator::Validate;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Identifiable)]
#[table_name = "users"]
pub struct UserId(#[column_name = "id"] pub i32);

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub email_address: String,
    pub use_email_address_for_gravatar: bool,
    pub gravatar_alternative: String,
}

impl User {
    pub fn by_id(conn: &PgConnection, id: UserId) -> QueryResult<Option<User>> {
        users::table.find(id.0).first(conn).optional()
    }

    pub fn by_username(conn: &PgConnection, username: &str) -> QueryResult<Option<User>> {
        users::table
            .filter(users::username.ilike(username))
            .first(conn)
            .optional()
    }

    pub fn by_email_address(conn: &PgConnection, email_address: &str) -> QueryResult<Option<User>> {
        users::table
            .filter(users::email_address.ilike(email_address))
            .first(conn)
            .optional()
    }

    pub fn get_id(&self) -> UserId {
        UserId(self.id)
    }
}

#[derive(Insertable, Debug, Default, Validate)]
#[table_name = "users"]
pub struct NewUser {
    #[validate(length(min = 1, max = 40))]
    pub username: String,
    #[validate(length(min = 1, max = 40))]
    pub display_name: String,
    pub password_hash: String,

    #[validate(length(max = 255))]
    #[validate(email)]
    pub email_address: String,
    use_email_address_for_gravatar: bool,
    gravatar_alternative: String,
}

impl NewUser {
    pub fn new(username: String, password_hash: String, email_address: String) -> Self {
        NewUser {
            username: username.to_lowercase(),
            display_name: username,
            password_hash,

            // TODO: in principle, only the host-part of the email address should be lowercased.
            email_address: email_address.to_lowercase(),

            use_email_address_for_gravatar: true,
            gravatar_alternative: random_string::unambiguous_string(32),
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Option<User>> {
        use crate::schema::users::dsl::*;

        conn.transaction(|| {
            let maybe_inserted = diesel::insert_into(users)
                .values(self)
                .on_conflict_do_nothing()
                .get_result::<User>(conn)
                .optional()?;

            Ok(maybe_inserted)
        })
    }
}
