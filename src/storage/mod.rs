use std::fmt::Debug;

use crate::graph::{edge::DelfEdge, object::DelfObject};

mod diesel;

pub trait DelfStorageConnection: Debug {
    fn connect(database_url: &str) -> Self
    where
        Self: Sized;

    fn get_object_ids(
        &self,
        from_id: i64,
        edge_field: &String,
        table: &String,
        id_field: &String,
    ) -> Vec<i64>;

    fn delete_edge(&self, to: &DelfObject, from_id: i64, to_id: Option<i64>, edge: &DelfEdge);

    fn delete_object(&self, obj: &DelfObject, id: i64);

    fn validate_edge(&self, edge: &DelfEdge) -> Result<(), String>;

    fn validate_object(&self, obj: &DelfObject) -> Result<(), String>;
}

pub fn get_connection(plugin: &str, url: &str) -> Box<dyn DelfStorageConnection> {
    match plugin {
        "diesel" => Box::new(diesel::DieselConnection::connect(url)),
        _ => panic!("no DelfStorageConnection with that name"),
    }
}
