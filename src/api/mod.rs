use rocket::{delete, State};

use crate::graph::DelfGraph;
use crate::DelfYamls;

#[delete("/object/<object_type>/<id>")]
pub fn delete_object(object_type: String, id: String, yamls: State<DelfYamls>) {
    let graph = DelfGraph::new(&yamls);
    graph.delete_object(&object_type, &id);
}

#[delete("/edge/<edge_type>/<from_id>/<to_id>")]
pub fn delete_edge(edge_type: String, from_id: String, to_id: String, yamls: State<DelfYamls>) {
    let graph = DelfGraph::new(&yamls);
    graph.delete_edge(&edge_type, &from_id, &to_id);
}
