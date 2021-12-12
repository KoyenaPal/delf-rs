use std::fmt::Debug;

use crate::graph::{edge::DelfEdge, object::DelfObject};
use std::collections::HashSet;

mod diesel;

/// Trait defining the api for a DelF storage connection.
pub trait DelfStorageConnection: Debug {
    /// Connect to the storage via a connection string
    fn connect(database_url: &str) -> Self
    where
        Self: Sized;

    /// Get the object ids of the object an edge points to.
    fn get_object_ids(
        &self,
        from_id: &String,
        from_id_type: &String,
        edge_field: &String,
        table: &String,
        id_field: &String,
        id_type: &String,
    ) -> Vec<String>;

    /// Get the object ids that have a `time_field` with a value before now.
    fn get_object_ids_by_time(
        &self,
        table: &String,
        time_field: &String,
        id_field: &String,
        id_type: &String,
    ) -> Vec<String>;

    fn get_object_ids_by_list(
        &self,
        from_id_list: &HashSet<String>,
        from_id_type: &String,
        edge_field: &String,
        table: &String,
        id_list: &String,
        id_type: &String,
    ) -> Vec<String>;
    

    /// Delete an edge instance.
    fn delete_edge(
        &self,
        to: &DelfObject,
        from_id_type: &String,
        from_id: &String,
        to_id: Option<&String>,
        edge: &DelfEdge,
    ) -> bool;

    /// Delete an object instance.
    fn delete_object(&self, obj: &DelfObject, id: &String) -> bool;

    /// Validate the edge exists in the storage as described in the struct
    fn validate_edge(&self, edge: &DelfEdge) -> Result<(), String>;

    /// Validate the object exists in the storage as described in the struct
    fn validate_object(&self, obj: &DelfObject) -> Result<(), String>;

    /// Check if an inbound edge exists for a given object instance.
    fn has_edge(&self, obj: &DelfObject, id: &String, edge: &DelfEdge) -> bool;
}

/// Given the name of the plugin (trait implementor) and the connection string, return an instance of that DelfStorageConnection.
pub fn get_connection(plugin: &str, url: &str) -> Box<dyn DelfStorageConnection> {
    match plugin {
        "diesel" => Box::new(diesel::DieselConnection::connect(url)),
        _ => panic!("no DelfStorageConnection with that name"),
    }
}
