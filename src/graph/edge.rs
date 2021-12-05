use yaml_rust::Yaml;

use crate::graph::DelfGraph;

/// The deletion types for a DelfEdge.  The type describes how the object the edge points to should be deleted by the DelfGraph.
#[derive(Clone, Debug, PartialEq)]
pub enum DeleteType {
    /// Delete the edge and object the edge refers to
    Deep,
    /// Delete the edge, but not the object it refers to
    Shallow,
    /// Delete the edge and delete the object if this edge is the last edge referring to it
    RefCount,
}

impl DeleteType {
    fn from(input: &str) -> DeleteType {
        match input.to_lowercase().as_str() {
            "deep" => DeleteType::Deep,
            "shallow" => DeleteType::Shallow,
            "refcount" => DeleteType::RefCount,
            _ => panic!("No edge type"),
        }
    }
}

/// Describes the object from the point of view of the edge
#[derive(Clone, Debug, PartialEq)]
pub struct ToType {
    pub object_type: String,
    pub field: String,
    pub mapping_table: Option<String>,
}

/// The DelfEdge contains the information about the edge as described in the schema
#[derive(Clone, Debug, PartialEq)]
pub struct DelfEdge {
    /// The unique name identifying the edge, used by the API to enable deletion of an edge directly.
    pub name: String,
    /// Describes the object the edge points to.
    pub to: ToType,
    /// Describes the id type of the object the edge points from.
    pub from_id_type: String,
    /// How should the deletion of this edge affect the object it points to.
    pub deletion: DeleteType,
    /// If this edge is deleted (typically, shallowly), is there an inverse edge that also needs to be deleted.
    pub inverse: Option<String>,
}

impl From<&Yaml> for DelfEdge {

    /// Create a DelfEdge from a yaml struct.  The keys `name`, `to` (which iteslf contains a yaml object with the fields `object_type`, `field`, and optionally `mapping_table`), and `deletion` are required.  An `inverse` key may also be specified.
    fn from(obj: &Yaml) -> DelfEdge {
        DelfEdge {
            name: String::from(obj["name"].as_str().unwrap()),
            to: ToType::from(&obj["to"]),
            from_id_type: String::from(obj["from_id_type"].as_str().unwrap()),
            deletion: DeleteType::from(obj["deletion"].as_str().unwrap()),
            inverse: match obj["inverse"].as_str() {
                Some(edge_name) => Some(String::from(edge_name)),
                None => None,
            }, // gets updated later if needed
        }
    }
}

impl From<&Yaml> for ToType {
    /// Create a ToType from a yaml struct. The expected keys are `object_type`, `field`, and optionally `mapping_table`.
    fn from(obj: &Yaml) -> ToType {
        ToType {
            object_type: String::from(obj["object_type"].as_str().unwrap()),
            field: String::from(obj["field"].as_str().unwrap()),
            mapping_table: match obj["mapping_table"].as_str() {
                Some(table) => Some(String::from(table)),
                None => None,
            },
        }
    }
}

impl DelfEdge {
    /// Delete a specific edge between two object instances
    pub fn delete_one(&self, from_id: &String, to_id: &String, graph: &DelfGraph) {
        let to_obj = graph.get_object(&self.to.object_type);
        let s = &*(graph.storages.get(&to_obj.storage).unwrap());

        match self.deletion {
            DeleteType::Deep => {
                graph._delete_object(&to_obj.name, to_id, Some(self));
            }
            DeleteType::RefCount => {
                let inbound_edges = graph.get_inbound_edges(to_obj);
                let mut last_ref = true;
                for inbound_edge in inbound_edges.iter() {
                    if *inbound_edge != self {
                        if s.has_edge(to_obj, to_id, inbound_edge) {
                            last_ref = false;
                            break;
                        }
                    }
                }

                if last_ref {
                    graph._delete_object(&to_obj.name, to_id, Some(self));
                }
            }
            DeleteType::Shallow => (),
        }

        let deleted = s.delete_edge(to_obj, &self.from_id_type, from_id, None, self);

        if deleted {
            println!("Edge deleted: {:#?}", self.name);
            match &self.inverse {
                Some(inverse) => {
                    graph.delete_edge(&inverse, to_id, from_id);
                }
                None => (),
            }
        }
    }

    /// Delete all edges of a given type from the instance of the object
    pub fn delete_all(&self, from_id: &String, from_id_type: &String, graph: &DelfGraph) {
        let to_obj = graph.get_object(&self.to.object_type);
        let s = &*(graph.storages.get(&to_obj.storage).unwrap());

        let table = match &self.to.mapping_table {
            Some(tbl) => tbl,
            None => &to_obj.name,
        };

        match self.deletion {
            DeleteType::Deep => {
                // collect object ids to delete
                let to_ids = s.get_object_ids(
                    from_id,
                    from_id_type,
                    &self.to.field,
                    table,
                    &to_obj.id_field,
                    &to_obj.id_type,
                );
                for to_id in to_ids.iter() {
                    graph._delete_object(&to_obj.name, to_id, Some(self));
                }
            }
            DeleteType::RefCount => {
                let to_ids = s.get_object_ids(
                    from_id,
                    from_id_type,
                    &self.to.field,
                    table,
                    &to_obj.id_field,
                    &to_obj.id_type,
                );
                let inbound_edges = graph.get_inbound_edges(to_obj);
                for to_id in to_ids.iter() {
                    let mut last_ref = true;
                    for inbound_edge in inbound_edges.iter() {
                        if *inbound_edge != self {
                            if s.has_edge(to_obj, to_id, inbound_edge) {
                                last_ref = false;
                                break;
                            }
                        }
                    }

                    if last_ref {
                        graph._delete_object(&to_obj.name, to_id, Some(self));
                    }
                }
            }
            _ => (),
        }

        match &self.inverse {
            Some(inverse) => {
                let table: &String;
                match &self.to.mapping_table {
                    Some(mapping_table) => {
                        table = &mapping_table;
                    }
                    None => {
                        table = &to_obj.name;
                    }
                }
                // collect object ids to delete
                let to_ids = s.get_object_ids(
                    from_id,
                    from_id_type,
                    &self.to.field,
                    table,
                    &to_obj.id_field,
                    &to_obj.id_type,
                );
                for to_id in to_ids.iter() {
                    graph.delete_edge(&inverse, to_id, from_id);
                }
            }
            None => (),
        }

        if s.delete_edge(to_obj, &self.from_id_type, from_id, None, self) {
            println!("Edges Deleted: {:#?}", self.name);
        }
    }

    /// Validate the edge exists in the storage as described in the schema
    pub fn validate(&self, graph: &DelfGraph) -> Result<(), String> {
        let to_obj = graph.get_object(&self.to.object_type);
        let s = &*(graph.storages.get(&to_obj.storage).unwrap());
        let res = s.validate_edge(self);
        return res;
    }
}
