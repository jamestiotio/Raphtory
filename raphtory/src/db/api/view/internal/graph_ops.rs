use crate::{
    core::{
        entities::{edges::edge_ref::EdgeRef, nodes::node_ref::NodeRef, LayerIds, EID, VID},
        Direction,
    },
    db::api::view::{
        internal::{Base, EdgeFilter},
        BoxedLIter,
    },
};

/// The GraphViewInternalOps trait provides a set of methods to query a directed graph
/// represented by the raphtory_core::tgraph::TGraph struct.
pub trait GraphOps<'graph>: Send + Sync {
    /// Check if a node exists and returns internal reference.
    fn internal_node_ref(
        &self,
        v: NodeRef,
        layer_ids: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> Option<VID>;

    fn find_edge_id(
        &self,
        e_id: EID,
        layer_ids: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> Option<EdgeRef>;

    /// Returns the total number of nodes in the graph.
    fn nodes_len(&self, layer_ids: LayerIds, filter: Option<&EdgeFilter>) -> usize;

    /// Returns the total number of edges in the graph.
    fn edges_len(&self, layers: LayerIds, filter: Option<&EdgeFilter>) -> usize;

    fn temporal_edges_len(&self, layers: LayerIds, filter: Option<&EdgeFilter>) -> usize;

    /// Returns true if the graph contains an edge between the source node
    /// (src) and the destination node (dst).
    /// # Arguments
    ///
    /// * `src` - The source node of the edge.
    /// * `dst` - The destination node of the edge.
    fn has_edge_ref(
        &self,
        src: VID,
        dst: VID,
        layers: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> bool {
        self.edge_ref(src, dst, layers, filter).is_some()
    }

    /// Returns true if the graph contains the specified node (v).
    /// # Arguments
    ///
    /// * `v` - NodeRef of the node to check.
    #[inline]
    fn has_node_ref(&self, v: NodeRef, layers: &LayerIds, filter: Option<&EdgeFilter>) -> bool {
        self.internal_node_ref(v, layers, filter).is_some()
    }

    /// Returns the number of edges that point towards or from the specified node
    /// (v) based on the direction (d).
    /// # Arguments
    ///
    /// * `v` - VID of the node to check.
    /// * `d` - Direction of the edges to count.
    fn degree(&self, v: VID, d: Direction, layers: &LayerIds, filter: Option<&EdgeFilter>)
        -> usize;

    /// Returns the VID that corresponds to the specified node ID (v).
    /// Returns None if the node ID is not present in the graph.
    /// # Arguments
    ///
    /// * `v` - The node ID to lookup.
    #[inline]
    fn node_ref(&self, v: u64, layers: &LayerIds, filter: Option<&EdgeFilter>) -> Option<VID> {
        self.internal_node_ref(v.into(), layers, filter)
    }

    /// Returns the edge reference that corresponds to the specified src and dst node
    /// # Arguments
    ///
    /// * `src` - The source node.
    /// * `dst` - The destination node.
    ///
    /// Returns:
    ///
    /// * `Option<EdgeRef>` - The edge reference if it exists.
    fn edge_ref(
        &self,
        src: VID,
        dst: VID,
        layer: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> Option<EdgeRef>;

    /// Returns all the node references in the graph.
    /// Returns:
    /// * `Box<dyn Iterator<Item = VID> + Send>` - An iterator over all the node
    /// references in the graph.
    fn node_refs(&self, layers: LayerIds, filter: Option<&EdgeFilter>) -> BoxedLIter<'graph, VID>;

    /// Returns all the edge references in the graph.
    ///
    /// Returns:
    ///
    /// * `Box<dyn Iterator<Item = EdgeRef> + Send>` - An iterator over all the edge references.
    fn edge_refs(
        &self,
        layers: LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> BoxedLIter<'graph, EdgeRef>;

    /// Returns an iterator over the edges connected to a given node in a given direction.
    ///
    /// # Arguments
    ///
    /// * `v` - A reference to the node for which the edges are being queried.
    /// * `d` - The direction in which to search for edges.
    /// * `layer` - The optional layer to consider
    ///
    /// Returns:
    ///
    /// Box<dyn Iterator<Item = EdgeRef> + Send> -  A boxed iterator that yields references to
    /// the edges connected to the node.
    fn node_edges(
        &self,
        v: VID,
        d: Direction,
        layer: LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> BoxedLIter<'graph, EdgeRef>;

    /// Returns an iterator over the neighbors of a given node in a given direction.
    ///
    /// # Arguments
    ///
    /// * `v` - A reference to the node for which the neighbors are being queried.
    /// * `d` - The direction in which to search for neighbors.
    ///
    /// Returns:
    ///
    /// A boxed iterator that yields references to the neighboring nodes.
    fn neighbours(
        &self,
        v: VID,
        d: Direction,
        layers: LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> BoxedLIter<'graph, VID>;
}

pub trait InheritGraphOps: Base + Send + Sync {}

impl<'base: 'graph, 'graph, G: InheritGraphOps + Send + Sync + ?Sized + 'graph> GraphOps<'graph>
    for G
where
    G::Base: GraphOps<'base>,
{
    #[inline]
    fn internal_node_ref(
        &self,
        v: NodeRef,
        layer_ids: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> Option<VID> {
        self.base().internal_node_ref(v, layer_ids, filter)
    }

    #[inline]
    fn find_edge_id(
        &self,
        e_id: EID,
        layer_ids: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> Option<EdgeRef> {
        self.base().find_edge_id(e_id, layer_ids, filter)
    }

    #[inline]
    fn nodes_len(&self, layer_ids: LayerIds, filter: Option<&EdgeFilter>) -> usize {
        self.base().nodes_len(layer_ids, filter)
    }

    #[inline]
    fn edges_len(&self, layers: LayerIds, filter: Option<&EdgeFilter>) -> usize {
        self.base().edges_len(layers, filter)
    }

    #[inline]
    fn temporal_edges_len(&self, layers: LayerIds, filter: Option<&EdgeFilter>) -> usize {
        self.base().temporal_edges_len(layers, filter)
    }

    #[inline]
    fn has_edge_ref(
        &self,
        src: VID,
        dst: VID,
        layers: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> bool {
        self.base().has_edge_ref(src, dst, layers, filter)
    }

    #[inline]
    fn has_node_ref(&self, v: NodeRef, layers: &LayerIds, filter: Option<&EdgeFilter>) -> bool {
        self.base().has_node_ref(v, layers, filter)
    }

    #[inline]
    fn degree(
        &self,
        v: VID,
        d: Direction,
        layers: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> usize {
        self.base().degree(v, d, layers, filter)
    }

    #[inline]
    fn node_ref(&self, v: u64, layers: &LayerIds, filter: Option<&EdgeFilter>) -> Option<VID> {
        self.base().node_ref(v, layers, filter)
    }

    #[inline]
    fn edge_ref(
        &self,
        src: VID,
        dst: VID,
        layer: &LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> Option<EdgeRef> {
        self.base().edge_ref(src, dst, layer, filter)
    }

    #[inline]
    fn node_refs(&self, layers: LayerIds, filter: Option<&EdgeFilter>) -> BoxedLIter<'graph, VID> {
        self.base().node_refs(layers, filter)
    }

    #[inline]
    fn edge_refs(
        &self,
        layers: LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> BoxedLIter<'graph, EdgeRef> {
        self.base().edge_refs(layers, filter)
    }

    #[inline]
    fn node_edges(
        &self,
        v: VID,
        d: Direction,
        layer: LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> BoxedLIter<'graph, EdgeRef> {
        self.base().node_edges(v, d, layer, filter)
    }

    #[inline]
    fn neighbours(
        &self,
        v: VID,
        d: Direction,
        layers: LayerIds,
        filter: Option<&EdgeFilter>,
    ) -> BoxedLIter<'graph, VID> {
        self.base().neighbours(v, d, layers, filter)
    }
}
