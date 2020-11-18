use diesel;
use diesel::sql_types::{BigInt, Integer, Text};
use diesel::Connection;
use diesel::QueryableByName;
use diesel::RunQueryDsl;

pub use super::DelfStorageConnection;
use crate::graph::{edge::DelfEdge, object::DelfObject};

pub struct DieselConnection {
    connection: diesel::mysql::MysqlConnection,
}

impl std::fmt::Debug for DieselConnection {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "A DieselConnection")
    }
}

#[derive(QueryableByName)]
struct ObjectIdIntResult {
    #[sql_type = "Integer"]
    id_field: i32,
}

#[derive(QueryableByName)]
struct ObjectIdStrResult {
    #[sql_type = "Text"]
    id_field: String,
}

#[derive(QueryableByName)]
struct ValidationResult {
    #[sql_type = "BigInt"]
    count: i64,
}

impl DelfStorageConnection for DieselConnection {
    fn connect(database_url: &str) -> DieselConnection {
        let raw_connection = diesel::mysql::MysqlConnection::establish(database_url);
        match raw_connection {
            Ok(conn) => DieselConnection { connection: conn },
            Err(_) => panic!("failed to connect to mysql"),
        }
    }

    fn get_object_ids(
        &self,
        from_id: &String,
        from_id_type: &String,
        edge_field: &String,
        table: &String,
        id_field: &String,
        id_type: &String,
    ) -> Vec<String> {
        let mut query_str = format!(
            "SELECT {} as id_field FROM {} WHERE {} = ",
            id_field, table, edge_field
        );
        match from_id_type.to_lowercase().as_str() {
            "string" => query_str.push_str(format!("'{}'", from_id).as_str()),
            "integer" => query_str.push_str(format!("{}", from_id).as_str()),
            _ => panic!("Unrecognized id type"),
        }

        let query = diesel::sql_query(query_str);

        let mut obj_ids = Vec::new();

        match id_type.to_lowercase().as_str() {
            "string" => {
                let res = query.load::<ObjectIdStrResult>(&self.connection).unwrap();
                for o_id in res {
                    obj_ids.push(o_id.id_field)
                }
            }
            "integer" => {
                let res = query.load::<ObjectIdIntResult>(&self.connection).unwrap();
                for o_id in res {
                    obj_ids.push(o_id.id_field.to_string())
                }
            }
            _ => panic!("Unrecognized id type"),
        }

        return obj_ids;
    }

    fn delete_edge(
        &self,
        to: &DelfObject,
        from_id: &String,
        to_id: Option<&String>,
        edge: &DelfEdge,
    ) {
        match &edge.to.mapping_table {
            Some(map_table) => self.delete_indirect_edge(edge, to, from_id, to_id, map_table), // delete the id pair from the mapping table
            None => self.delete_direct_edge(to, from_id, edge), // try to set null in object table
        }
    }

    fn delete_object(&self, obj: &DelfObject, id: &String) {
        let mut query_str = format!("DELETE FROM {} WHERE {} = ", obj.name, obj.id_field,);
        match obj.id_type.to_lowercase().as_str() {
            "string" => query_str.push_str(format!("'{}'", id).as_str()),
            "integer" => query_str.push_str(format!("{}", id).as_str()),
            _ => panic!("Unrecognized id type"),
        }

        diesel::sql_query(query_str)
            .execute(&self.connection)
            .unwrap();
        println!("deleted object!");
    }

    fn validate_edge(&self, edge: &DelfEdge) -> Result<(), String> {
        let table: &str;
        match &edge.to.mapping_table {
            Some(map_table) => {
                table = map_table;
            }
            None => {
                table = &edge.to.object_type;
            }
        }
        let res = diesel::sql_query(format!(
            "SELECT count({}) as count FROM {}",
            edge.to.field, table
        ))
        .load::<ValidationResult>(&self.connection);

        match res {
            Ok(_) => return Ok(()),
            Err(_) => return Err(format!("Edge {} doesn't match database schema", edge.name)),
        }
    }

    fn validate_object(&self, obj: &DelfObject) -> Result<(), String> {
        let res = diesel::sql_query(format!(
            "SELECT count({}) as count FROM {}",
            obj.id_field, obj.name
        ))
        .load::<ValidationResult>(&self.connection);

        match res {
            Ok(_) => return Ok(()),
            Err(_) => return Err(format!("Object {} doesn't match database schema", obj.name)),
        }
    }
}

impl DieselConnection {
    fn delete_indirect_edge(
        &self,
        edge: &DelfEdge,
        to: &DelfObject,
        from_id: &String,
        to_id: Option<&String>,
        table: &String,
    ) {
        match to_id {
            Some(id) => {
                diesel::sql_query(format!(
                    "DELETE FROM {} WHERE {} = {} AND {} = {}",
                    table, to.id_field, id, edge.to.field, from_id
                ))
                .execute(&self.connection)
                .unwrap();
            }
            None => {
                diesel::sql_query(format!(
                    "DELETE FROM {} WHERE {} = {}",
                    table, edge.to.field, from_id
                ))
                .execute(&self.connection)
                .unwrap();
            }
        }

        println!("deleted indirect edge {}!", table);
    }

    fn delete_direct_edge(&self, to: &DelfObject, from_id: &String, edge: &DelfEdge) {
        diesel::sql_query(format!(
            "UPDATE {} SET {} = NULL WHERE {} = {}",
            to.name, edge.to.field, edge.to.field, from_id
        ))
        .execute(&self.connection)
        .unwrap();
        println!("deleted direct edge!");
    }
}
