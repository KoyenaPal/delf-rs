use std::collections::{HashMap, HashSet};
use std::process::exit;

use ansi_term::Colour::{Red, Green, Cyan};
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    Directed, Graph, Incoming, Outgoing,
};

/// The edge of a DelfGraph is a DelfEdge
pub mod edge;
/// The node of a DelfGraph is a DelfObject
pub mod object;

use crate::storage::{get_connection, DelfStorageConnection};
use crate::DelfYamls;

/// The DelfGraph is the core structure for delf's functionality.  It contains the algorithm to traverse the graph, as well as metadata to perform the deletions.
#[derive(Debug)]
pub struct DelfGraph {
    pub(crate) nodes: HashMap<String, NodeIndex>,
    pub(crate) edges: HashMap<String, EdgeIndex>,
    graph: Graph<object::DelfObject, edge::DelfEdge, Directed>,
    storages: HashMap<String, Box<dyn DelfStorageConnection>>,
}

impl DelfGraph {
    /// Create a new DelfGraph from a schema and a config.  See [yaml_rust](../../yaml_rust/index.html) for information on creating the Yaml structs, or alternately use the helper functions: [read_files](../fn.read_files.html), [read_yamls](../fn.read_yamls.html) for constructing a DelfGraph from either paths or `&str` of yaml.
    pub fn new(yamls: &DelfYamls) -> DelfGraph {
        let schema = &yamls.schema;
        let config = &yamls.config;
        let mut edges_to_insert = Vec::new();
        let mut nodes = HashMap::<String, NodeIndex>::new();
        let mut edges = HashMap::<String, EdgeIndex>::new();

        let mut graph = Graph::<object::DelfObject, edge::DelfEdge>::new();

        // each yaml is an object
        for yaml in schema.iter() {
            let obj_name = String::from(yaml["object_type"]["name"].as_str().unwrap());
            let obj_node = object::DelfObject::from(&yaml["object_type"]);

            let node_id = graph.add_node(obj_node);
            nodes.insert(obj_name.clone(), node_id);

            // need to make sure all the nodes exist before edges can be added to the graph
            for e in yaml["object_type"]["edge_types"].as_vec().unwrap().iter() {
                let delf_edge = edge::DelfEdge::from(e);
                edges_to_insert.push((obj_name.clone(), delf_edge));
            }
        }

        // add all the edges to the graph
        for (from, e) in edges_to_insert.iter_mut() {
            if !nodes.contains_key(&e.to.object_type) {
                eprintln!("Error creating edge {:#?}: No object with name {:#?}", e.name, e.to.object_type);
                exit(1);
            }
            let edge_id = graph.add_edge(nodes[from], nodes[&e.to.object_type], e.clone());
            edges.insert(String::from(&e.name), edge_id);
        }

        // create the storage map
        let mut storages = HashMap::<String, Box<dyn DelfStorageConnection>>::new();

        for yaml in config.iter() {
            for storage in yaml["storages"].as_vec().unwrap().iter() {
                let storage_name = String::from(storage["name"].as_str().unwrap());
                storages.insert(
                    storage_name,
                    get_connection(
                        storage["plugin"].as_str().unwrap(),
                        storage["url"].as_str().unwrap(),
                    ),
                );
            }
        }

        return DelfGraph {
            nodes,
            edges,
            graph,
            storages,
        };
    }

    /// Pretty print the graph's contents.
    pub fn print(&self) {
        println!("{:#?}", self.graph);
    }

    /// Given an edge name, get the corresponding DelfEdge
    pub fn get_edge(&self, edge_name: &String) -> &edge::DelfEdge {
        let edge_id = self.edges.get(edge_name).unwrap();
        return self.graph.edge_weight(*edge_id).unwrap();
    }

    /// Given an edge name and the ids of the to/from object instances, delete the edge
    pub fn delete_edge(&self, edge_name: &String, from_id: &String, to_id: &String) {
        let e = self.get_edge(edge_name);
        e.delete_one(from_id, to_id, self);
    }

    /// Given an object name, get the corresponding DelfObject
    pub fn get_object(&self, object_name: &String) -> &object::DelfObject {
        let object_id = self.nodes.get(object_name).unwrap();
        return self.graph.node_weight(*object_id).unwrap();
    }

    /// Given the object name and the id of the instance, delete the object
    pub fn delete_object(&self, object_name: &String, id: &String) {
        self._delete_object(object_name, id, None);
    }

    fn _delete_object(
        &self,
        object_name: &String,
        id: &String,
        from_edge: Option<&edge::DelfEdge>,
    ) {
        let obj = self.get_object(object_name);

        let deleted = obj.delete(id, from_edge, &self.storages);
        if deleted {
            let edges = self.graph.edges_directed(self.nodes[&obj.name], Outgoing);
            for e in edges {
                e.weight().delete_all(id, &obj.id_type, &obj.id_list, self);
            }
        }
    }

    /// Validate that the objects and edges described in the schema exist in the corresponding storage as expected.  Additionally, ensure that all objects in the graph are reachable by traversal via `deep` or `refcount` edges starting at an object with deletion type of `directly`, `directly_only`, `short_ttl`, or `not_deleted`.  This ensures that all objects are deletable and accounted for.
    pub fn validate(&self) {
        println!("\u{1f50d} {}", Cyan.bold().paint("Validating DelF graph..."));

        let mut errs = Vec::new();
        let mut passed = true;
        for (_, node_id) in self.nodes.iter() {
            match self.graph
                .node_weight(*node_id)
                .unwrap()
                .validate(&self.storages) {
                    Err(e) => errs.push(e),
                    _ => ()
                }
        }

        if errs.len() > 0 {
            passed = false;
            println!("\u{274c} {}", Red.paint("Not all objects found in storage"));
            for err in errs.drain(..) {
                println!("  {}", err);
            }
        } else {
            println!("\u{2705} {}", Green.paint("Objects exist in storage"));
        }

        for (_, edge_id) in self.edges.iter() {
            match self.graph.edge_weight(*edge_id).unwrap().validate(self) {
                Err(e) => errs.push(e),
                _ => ()
            }
        }

        if errs.len() > 0 {
            passed = false;
            println!("\u{274c} {}", Red.paint("Not all edges found in storage"));
            for err in errs.drain(..) {
                println!("  {}", err);
            }
        } else {
            println!("\u{2705} {}", Green.paint("Edges exist in storage"));
        }

        match self.reachability_analysis() {
            Err(e) => errs.push(e),
            _ => ()
        }

        if errs.len() > 0 {
            passed = false;
            println!("\u{274c} {}", Red.paint("Not all objects deletable"));
            for err in errs.drain(..) {
                println!("  {}", err);
            }
        } else {
            println!("\u{2705} {}", Green.paint("All objects deletable"));
        }

        if passed {
            println!("\u{1F680} {} \u{1F680}", Green.bold().paint("Validation successful!"));
        } else {
            println!("\u{26a0} {} \u{26a0}", Red.bold().paint("Validation errors found"));
        }
    }

    // Starting from a directly deletable (or excepted) node, ensure all ndoes are reached.
    fn reachability_analysis(&self) -> Result<(), String> {
        let mut visited_nodes = HashSet::new();
        for (_, node_id) in self.nodes.iter() {
            let obj = self.graph.node_weight(*node_id).unwrap();
            match obj.deletion {
                object::DeleteType::ShortTTL
                | object::DeleteType::Directly
                | object::DeleteType::DirectlyOnly
                | object::DeleteType::NotDeleted => {
                    // this object is a starting point in traversal, start traversal
                    self.visit_node(&obj.name, &mut visited_nodes);
                }
                _ => (),
            }
        }

        if visited_nodes.len() != self.nodes.len() {
            let node_set: HashSet<String> = self.nodes.keys().cloned().collect();
            return Err(format!(
                "Not all objects are deletable: {:?}",
                node_set.difference(&visited_nodes)
            ));
        } else {
            return Ok(());
        }
    }

    // Recursively visit all un-visited nodes that are connected via depp or refcounte edges from the starting node with the passed in name
    fn visit_node(&self, name: &String, visited_nodes: &mut HashSet<String>) {
        visited_nodes.insert(name.clone());

        let edges = self.graph.edges_directed(self.nodes[name], Outgoing);
        for e in edges {
            let ew = e.weight();
            match ew.deletion {
                edge::DeleteType::Deep | edge::DeleteType::RefCount => {
                    if !visited_nodes.contains(&ew.to.object_type) {
                        self.visit_node(&ew.to.object_type, visited_nodes);
                    }
                }
                _ => (),
            }
        }
    }

    // find all the inbound edges for a given object
    fn get_inbound_edges(&self, obj: &object::DelfObject) -> Vec<&edge::DelfEdge> {
        let object_id = self.nodes.get(&obj.name).unwrap();
        let edges = self.graph.edges_directed(*object_id, Incoming);
        let mut res = Vec::new();
        for edge in edges {
            res.push(edge.weight());
        }

        return res;
    }

    /// Check all objects in the DelfGraph with the deletion type of `short_ttl` if there are instances of the object which are past their expiration time.  If so, delete the objects.
    pub fn check_short_ttl(&self) {
        for (_, node_id) in self.nodes.iter() {
            let obj = self.graph.node_weight(*node_id).unwrap();

            for obj_id in obj.check_short_ttl(&self.storages).iter() {
                self.delete_object(&obj.name, obj_id);
            }
        }
    }
}
