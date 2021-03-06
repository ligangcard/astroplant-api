use crate::schema::{peripheral_definitions, peripherals};

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::{Identifiable, QueryResult, Queryable};
use validator::Validate;

#[rustfmt::skip]
use super::{
    Kit, KitId,
    KitConfiguration, KitConfigurationId,
    PeripheralDefinition, PeripheralDefinitionId,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Identifiable)]
#[table_name = "peripherals"]
pub struct PeripheralId(#[column_name = "id"] pub i32);

#[derive(Clone, Debug, PartialEq, Queryable, Identifiable, Associations, AsChangeset, Validate)]
#[belongs_to(parent = "Kit", foreign_key = "kit_id")]
#[belongs_to(parent = "KitId", foreign_key = "kit_id")]
#[belongs_to(parent = "KitConfiguration", foreign_key = "kit_configuration_id")]
#[belongs_to(parent = "KitConfigurationId", foreign_key = "kit_configuration_id")]
#[belongs_to(
    parent = "PeripheralDefinition",
    foreign_key = "peripheral_definition_id"
)]
#[belongs_to(
    parent = "PeripheralDefinitionId",
    foreign_key = "peripheral_definition_id"
)]
#[table_name = "peripherals"]
pub struct Peripheral {
    pub id: i32,
    pub kit_id: i32,
    pub kit_configuration_id: i32,
    pub peripheral_definition_id: i32,
    #[validate(length(min = 1, max = 40))]
    pub name: String,
    pub configuration: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Queryable, Identifiable, AsChangeset, Validate)]
#[table_name = "peripherals"]
pub struct UpdatePeripheral {
    pub id: i32,
    // None means don't update.
    #[validate(length(min = 1, max = 40))]
    pub name: Option<String>,
    pub configuration: Option<serde_json::Value>,
}

impl Peripheral {
    pub fn by_id(conn: &PgConnection, peripheral_id: PeripheralId) -> QueryResult<Option<Self>> {
        peripherals::table
            .find(&peripheral_id.0)
            .first(conn)
            .optional()
    }

    pub fn delete(&self, conn: &PgConnection) -> QueryResult<bool> {
        diesel::delete(self).execute(conn).map(|r| r > 0)
    }

    pub fn peripherals_of_kit(conn: &PgConnection, kit: &Kit) -> QueryResult<Vec<Self>> {
        Peripheral::belonging_to(kit).load(conn)
    }

    pub fn peripherals_of_kit_id(conn: &PgConnection, kit_id: KitId) -> QueryResult<Vec<Self>> {
        Peripheral::belonging_to(&kit_id).load(conn)
    }

    pub fn peripherals_of_kit_configuration(
        conn: &PgConnection,
        kit_configuration: &KitConfiguration,
    ) -> QueryResult<Vec<Self>> {
        Peripheral::belonging_to(kit_configuration).load(conn)
    }

    pub fn peripherals_of_kit_configuration_id(
        conn: &PgConnection,
        kit_configuration_id: KitConfigurationId,
    ) -> QueryResult<Vec<Self>> {
        Peripheral::belonging_to(&kit_configuration_id).load(conn)
    }

    pub fn peripherals_with_definitions_of_kit_configuration(
        conn: &PgConnection,
        kit_configuration: &KitConfiguration,
    ) -> QueryResult<Vec<(Self, PeripheralDefinition)>> {
        Peripheral::peripherals_with_definitions_of_kit_configuration_id(
            conn,
            kit_configuration.get_id(),
        )
    }

    pub fn peripherals_with_definitions_of_kit_configuration_id(
        conn: &PgConnection,
        kit_configuration_id: KitConfigurationId,
    ) -> QueryResult<Vec<(Self, PeripheralDefinition)>> {
        Peripheral::belonging_to(&kit_configuration_id)
            .inner_join(peripheral_definitions::table)
            .load(conn)
    }

    pub fn get_id(&self) -> PeripheralId {
        PeripheralId(self.id)
    }
}

impl UpdatePeripheral {
    pub fn update(&self, conn: &PgConnection) -> QueryResult<Peripheral> {
        self.save_changes(conn)
    }
}

#[derive(Clone, Debug, PartialEq, Insertable, Validate)]
#[table_name = "peripherals"]
pub struct NewPeripheral {
    pub kit_id: i32,
    pub kit_configuration_id: i32,
    pub peripheral_definition_id: i32,
    #[validate(length(min = 1, max = 40))]
    pub name: String,
    pub configuration: serde_json::Value,
}

impl NewPeripheral {
    pub fn new(
        kit_id: KitId,
        kit_configuration_id: KitConfigurationId,
        peripheral_definition_id: PeripheralDefinitionId,
        name: String,
        configuration: serde_json::Value,
    ) -> Self {
        Self {
            kit_id: kit_id.0,
            kit_configuration_id: kit_configuration_id.0,
            peripheral_definition_id: peripheral_definition_id.0,
            name,
            configuration,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Peripheral> {
        use crate::schema::peripherals::dsl::*;

        diesel::insert_into(peripherals)
            .values(self)
            .on_conflict_do_nothing()
            .get_result::<Peripheral>(conn)
    }
}
