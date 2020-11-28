use yaml_rust::Yaml;

use crate::graph::DelfGraph;

/// The deletion types for a DelfEdge
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
    pub name: String,
    pub to: ToType,
    pub deletion: DeleteType,
    pub inverse: Option<String>,
}

impl From<&Yaml> for DelfEdge {
    fn from(obj: &Yaml) -> DelfEdge {
        DelfEdge {
            name: String::from(obj["name"].as_str().unwrap()),
            to: ToType::from(&obj["to"]),
            deletion: DeleteType::from(obj["deletion"].as_str().unwrap()),
            inverse: match obj["inverse"].as_str() {
                Some(edge_name) => Some(String::from(edge_name)),
                None => None,
            }, // gets updated later if needed
        }
    }
}

impl From<&Yaml> for ToType {
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
        println!("=======\ndeleting {:#?}", self.name);
        let to_obj = graph.get_object(&self.to.object_type);
        let s = &*(graph.storages.get(&to_obj.storage).unwrap());

        match self.deletion {
            DeleteType::Deep => {
                println!("    deep deletion, following to {}", self.to.object_type);
                graph._delete_object(&to_obj.name, to_id, Some(self));
            }
            DeleteType::RefCount => {
                let inbound_edges = graph.get_inbound_edges(to_obj);
                println!("{:?}", inbound_edges);
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
            DeleteType::Shallow => println!("    shallow deletion, not deleting object"),
        }

        let deleted = s.delete_edge(to_obj, from_id, None, self);

        if deleted {
            match &self.inverse {
                Some(inverse) => {
                    println!("    need to delete a reverse edge too!");
                    graph.delete_edge(&inverse, to_id, from_id);
                }
                None => (),
            }
        }
    }

    /// Delete all edges of a given type from the instance of the object
    pub fn delete_all(&self, from_id: &String, from_id_type: &String, graph: &DelfGraph) {
        println!("=======\ndeleting {:#?}", self.name);
        let to_obj = graph.get_object(&self.to.object_type);
        let s = &*(graph.storages.get(&to_obj.storage).unwrap());

        let table = match &self.to.mapping_table {
            Some(tbl) => tbl,
            None => &to_obj.name
        };

        match self.deletion {
            DeleteType::Deep => {
                println!("    deep deletion, following to {}", self.to.object_type);
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
                                println!("has inbound edge {:?}", inbound_edge);
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
            _ => println!("    shallow deletion, not deleting object"), // TODO: refcount
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
                println!("    need to delete a reverse edge too!");
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

        s.delete_edge(to_obj, from_id, None, self);
    }

    /// Validate the edge exists in the storage as described in the schema
    pub fn validate(&self, graph: &DelfGraph) -> Result<(), String> {
        let to_obj = graph.get_object(&self.to.object_type);
        let s = &*(graph.storages.get(&to_obj.storage).unwrap());
        let res = s.validate_edge(self);
        return res;
    }
}
