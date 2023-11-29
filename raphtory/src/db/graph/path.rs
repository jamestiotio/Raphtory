use crate::{
    core::entities::{edges::edge_ref::EdgeRef, LayerIds, VID},
    db::{
        api::{
            properties::Properties,
            view::{
                internal::{InternalLayerOps, OneHopFilter},
                BaseVertexViewOps, BoxedLIter, IntoDynBoxed, Layer,
            },
        },
        graph::{edge::EdgeView, vertex::VertexView},
    },
    prelude::*,
};
use std::sync::Arc;

pub(crate) type Operation<'a> = Arc<dyn Fn(VID) -> BoxedLIter<'a, VID> + Send + Sync + 'a>;
#[derive(Clone)]
pub struct PathFromGraph<'graph, G, GH> {
    pub(crate) graph: GH,
    pub(crate) base_graph: G,
    pub(crate) op: Operation<'graph>,
}

impl<'graph, G: GraphViewOps<'graph>> PathFromGraph<'graph, G, G> {
    pub fn new<OP: Fn(VID) -> BoxedLIter<'graph, VID> + Send + Sync + 'graph>(
        graph: G,
        op: OP,
    ) -> Self {
        let base_graph = graph.clone();
        let op: Operation<'graph> = Arc::new(op);
        PathFromGraph {
            graph,
            base_graph,
            op,
        }
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> PathFromGraph<'graph, G, GH> {
    fn base_iter(&self) -> BoxedLIter<'graph, VID> {
        self.graph
            .vertex_refs(self.graph.layer_ids(), self.graph.edge_filter())
    }

    pub fn iter(&self) -> impl Iterator<Item = PathFromVertex<'graph, G, GH>> + Send + 'graph {
        let graph = self.graph.clone();
        let base_graph = self.base_graph.clone();
        let op = self.op.clone();
        self.base_iter().map(move |vertex| {
            PathFromVertex::new_one_hop_filtered(
                base_graph.clone(),
                graph.clone(),
                vertex,
                op.clone(),
            )
        })
    }

    pub fn iter_refs(&self) -> impl Iterator<Item = BoxedLIter<'graph, VID>> + Send + 'graph {
        let op = self.op.clone();
        self.base_iter().map(move |vid| op(vid))
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> InternalLayerOps
    for PathFromGraph<'graph, G, GH>
{
    fn layer_ids(&self) -> LayerIds {
        self.graph.layer_ids()
    }

    fn layer_ids_from_names(&self, key: Layer) -> LayerIds {
        self.graph.layer_ids_from_names(key)
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> BaseVertexViewOps<'graph>
    for PathFromGraph<'graph, G, GH>
{
    type BaseGraph = G;
    type Graph = GH;
    type ValueType<T: 'graph> = BoxedLIter<'graph, BoxedLIter<'graph, T>>;
    type PropType = VertexView<GH, GH>;
    type PathType = PathFromGraph<'graph, G, G>;
    type Edge = EdgeView<G, GH>;
    type EList = BoxedLIter<'graph, BoxedLIter<'graph, EdgeView<G, GH>>>;

    fn map<O: 'graph, F: for<'a> Fn(&'a Self::Graph, VID) -> O + Send + Clone + 'graph>(
        &self,
        op: F,
    ) -> Self::ValueType<O> {
        let graph = self.graph.clone();
        self.iter_refs()
            .map(move |it| {
                let graph = graph.clone();
                let op = op.clone();
                it.map(move |vertex| op(&graph, vertex)).into_dyn_boxed()
            })
            .into_dyn_boxed()
    }

    fn as_props(&self) -> Self::ValueType<Properties<Self::PropType>> {
        self.map(|g, v| Properties::new(VertexView::new_internal(g.clone(), v)))
    }

    fn map_edges<
        I: Iterator<Item = EdgeRef> + Send + 'graph,
        F: for<'a> Fn(&'a Self::Graph, VID) -> I + Send + Sync + Clone + 'graph,
    >(
        &self,
        op: F,
    ) -> Self::EList {
        let graph = self.graph.clone();
        let base_graph = self.base_graph.clone();
        self.iter_refs()
            .map(move |it| {
                let base_graph = base_graph.clone();
                let graph = graph.clone();
                let op = op.clone();
                it.flat_map(move |vertex| {
                    let base_graph = base_graph.clone();
                    let graph = graph.clone();
                    op(&graph, vertex).map(move |edge| {
                        EdgeView::new_filtered(base_graph.clone(), graph.clone(), edge)
                    })
                })
                .into_dyn_boxed()
            })
            .into_dyn_boxed()
    }

    fn hop<
        I: Iterator<Item = VID> + Send + 'graph,
        F: for<'a> Fn(&'a Self::Graph, VID) -> I + Send + Sync + Clone + 'graph,
    >(
        &self,
        op: F,
    ) -> Self::PathType {
        let old_op = self.op.clone();
        let graph = self.graph.clone();
        PathFromGraph::new(self.base_graph.clone(), move |v| {
            let op = op.clone();
            let graph = graph.clone();
            Box::new(old_op(v).flat_map(move |vv| op(&graph, vv)))
        })
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> IntoIterator
    for PathFromGraph<'graph, G, GH>
{
    type Item = PathFromVertex<'graph, G, GH>;
    type IntoIter = BoxedLIter<'graph, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let graph = self.graph;
        let base_graph = self.base_graph;
        let op = self.op;
        graph
            .vertex_refs(graph.layer_ids(), graph.edge_filter())
            .map(move |vertex| {
                PathFromVertex::new_one_hop_filtered(
                    base_graph.clone(),
                    graph.clone(),
                    vertex,
                    op.clone(),
                )
            })
            .into_dyn_boxed()
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> OneHopFilter<'graph>
    for PathFromGraph<'graph, G, GH>
{
    type Graph = GH;
    type Filtered<GHH: GraphViewOps<'graph>> = PathFromGraph<'graph, G, GHH>;

    fn current_filter(&self) -> &Self::Graph {
        &self.graph
    }

    fn one_hop_filtered<GHH: GraphViewOps<'graph>>(
        &self,
        filtered_graph: GHH,
    ) -> Self::Filtered<GHH> {
        let base_graph = self.base_graph.clone();
        let op = self.op.clone();
        PathFromGraph {
            graph: filtered_graph,
            base_graph,
            op,
        }
    }
}

#[derive(Clone)]
pub struct PathFromVertex<'graph, G, GH> {
    pub graph: GH,
    pub(crate) base_graph: G,
    pub vertex: VID,
    pub(crate) op: Operation<'graph>,
}

impl<'graph, G: GraphViewOps<'graph>> PathFromVertex<'graph, G, G> {
    pub(crate) fn new<OP: Fn(VID) -> BoxedLIter<'graph, VID> + Send + Sync + 'graph>(
        graph: G,
        vertex: VID,
        op: OP,
    ) -> PathFromVertex<'graph, G, G> {
        let base_graph = graph.clone();
        let op: Operation = Arc::new(op);
        PathFromVertex {
            base_graph,
            graph,
            vertex,
            op,
        }
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> PathFromVertex<'graph, G, GH> {
    pub fn iter_refs(&self) -> BoxedLIter<'graph, VID> {
        let op = &self.op;
        op(self.vertex)
    }

    pub fn iter(&self) -> BoxedLIter<'graph, VertexView<G, GH>> {
        let graph = self.graph.clone();
        let base_graph = self.base_graph.clone();
        let iter = self.iter_refs().map(move |vertex| {
            VertexView::new_one_hop_filtered(base_graph.clone(), graph.clone(), vertex)
        });
        Box::new(iter)
    }

    pub(crate) fn new_one_hop_filtered(
        base_graph: G,
        graph: GH,
        vertex: VID,
        op: Operation<'graph>,
    ) -> Self {
        Self {
            base_graph,
            graph,
            vertex,
            op,
        }
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> InternalLayerOps
    for PathFromVertex<'graph, G, GH>
{
    fn layer_ids(&self) -> LayerIds {
        self.graph.layer_ids()
    }

    fn layer_ids_from_names(&self, key: Layer) -> LayerIds {
        self.graph.layer_ids_from_names(key)
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> BaseVertexViewOps<'graph>
    for PathFromVertex<'graph, G, GH>
{
    type BaseGraph = G;
    type Graph = GH;
    type ValueType<T: 'graph> = BoxedLIter<'graph, T>;
    type PropType = VertexView<GH, GH>;
    type PathType = PathFromVertex<'graph, G, G>;
    type Edge = EdgeView<G, GH>;
    type EList = BoxedLIter<'graph, EdgeView<G, GH>>;

    fn map<O: 'graph, F: for<'a> Fn(&'a Self::Graph, VID) -> O + Send + 'graph>(
        &self,
        op: F,
    ) -> Self::ValueType<O> {
        let graph = self.graph.clone();
        Box::new(self.iter_refs().map(move |vertex| op(&graph, vertex)))
    }

    fn as_props(&self) -> Self::ValueType<Properties<Self::PropType>> {
        self.map(|g, v| Properties::new(VertexView::new_internal(g.clone(), v)))
    }

    fn map_edges<
        I: Iterator<Item = EdgeRef> + Send + 'graph,
        F: for<'a> Fn(&'a Self::Graph, VID) -> I + Send + Sync + 'graph,
    >(
        &self,
        op: F,
    ) -> Self::EList {
        let graph = self.graph.clone();
        let base_graph = self.base_graph.clone();
        Box::new(self.iter_refs().flat_map(move |vertex| {
            let base_graph = base_graph.clone();
            let graph = graph.clone();
            op(&graph, vertex)
                .map(move |edge| EdgeView::new_filtered(base_graph.clone(), graph.clone(), edge))
        }))
    }

    fn hop<
        I: Iterator<Item = VID> + Send + 'graph,
        F: for<'a> Fn(&'a Self::Graph, VID) -> I + Send + Sync + Clone + 'graph,
    >(
        &self,
        op: F,
    ) -> Self::PathType {
        let old_op = self.op.clone();
        let graph = self.graph.clone();

        PathFromVertex::new(self.base_graph.clone(), self.vertex, move |v| {
            let graph = graph.clone();
            let op = op.clone();
            Box::new(old_op(v).flat_map(move |vv| op(&graph, vv)))
        })
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> IntoIterator
    for PathFromVertex<'graph, G, GH>
{
    type Item = VertexView<G, GH>;
    type IntoIter = BoxedLIter<'graph, VertexView<G, GH>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'graph, G: GraphViewOps<'graph>, GH: GraphViewOps<'graph>> OneHopFilter<'graph>
    for PathFromVertex<'graph, G, GH>
{
    type Graph = GH;
    type Filtered<GHH: GraphViewOps<'graph>> = PathFromVertex<'graph, G, GHH>;

    fn current_filter(&self) -> &Self::Graph {
        &self.graph
    }

    fn one_hop_filtered<GHH: GraphViewOps<'graph>>(
        &self,
        filtered_graph: GHH,
    ) -> Self::Filtered<GHH> {
        let base_graph = self.base_graph.clone();
        PathFromVertex {
            base_graph,
            graph: filtered_graph,
            vertex: self.vertex,
            op: self.op.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[test]
    fn test_vertex_view_ops() {
        let g = Graph::new();

        g.add_edge(0, 1, 2, NO_PROPS, None).unwrap();

        let n = Vec::from_iter(g.vertex(1).unwrap().neighbours().id());
        assert_eq!(n, [2])
    }
}
