use crate::model::schema::{
    get_node_type, merge_schemas, property_schema::PropertySchema, SchemaAggregate,
};
use dynamic_graphql::{ResolvedObject, ResolvedObjectFields};
use itertools::Itertools;
use raphtory::{
    db::{api::view::StaticGraphViewOps, graph::edge::EdgeView},
    prelude::EdgeViewOps,
};
use std::collections::{HashMap, HashSet};

#[derive(ResolvedObject)]
pub(crate) struct EdgeSchema<G: StaticGraphViewOps> {
    graph: G,
    src_type: String,
    dst_type: String,
}

impl<G: StaticGraphViewOps> EdgeSchema<G> {
    pub fn new(graph: G, src_type: String, dst_type: String) -> Self {
        Self {
            graph,
            src_type,
            dst_type,
        }
    }
}

#[ResolvedObjectFields]
impl<G: StaticGraphViewOps> EdgeSchema<G> {
    /// Returns the type of source for these edges
    async fn src_type(&self) -> String {
        self.src_type.clone()
    }

    /// Returns the type of destination for these edges
    async fn dst_type(&self) -> String {
        self.dst_type.clone()
    }

    /// Returns the list of property schemas for edges connecting these types of nodes
    async fn properties(&self) -> Vec<PropertySchema> {
        let filter_types = |edge: &EdgeView<G>| {
            let src_type = get_node_type(edge.src());
            let dst_type = get_node_type(edge.dst());
            src_type == self.src_type && dst_type == self.dst_type
        };

        let filtered_edges = self.graph.edges().filter(filter_types);

        let schema: SchemaAggregate = filtered_edges
            .map(collect_edge_schema)
            .reduce(merge_schemas)
            .unwrap_or_else(|| HashMap::new());

        schema.into_iter().map(|prop| prop.into()).collect_vec()
    }
}

fn collect_edge_schema<G: StaticGraphViewOps>(edge: EdgeView<G>) -> SchemaAggregate {
    edge.properties()
        .iter()
        .map(|(key, value)| (key.to_string(), HashSet::from([value.to_string()])))
        .collect()
}
