//! Defines the `ViewApi` trait, which represents the API for querying a view of the graph.

mod edge;
mod graph;
pub(crate) mod internal;
mod layer;
mod time;
mod vertex;

pub(crate) use edge::EdgeViewInternalOps;
pub use edge::{EdgeListOps, EdgeViewOps};

pub use graph::*;
pub use internal::{
    Base, BoxableGraphView, DynamicGraph, InheritViewOps, IntoDynamic, MaterializedGraph,
};
pub use layer::*;
pub use time::*;
pub(crate) use vertex::BaseVertexViewOps;
pub use vertex::{VertexListOps, VertexViewOps};

pub type BoxedIter<T> = Box<dyn Iterator<Item = T> + Send>;
pub type BoxedLIter<'a, T> = Box<dyn Iterator<Item = T> + Send + 'a>;

pub trait IntoDynBoxed<'a, T> {
    fn into_dyn_boxed(self) -> BoxedLIter<'a, T>;
}

impl<'a, T, I: Iterator<Item = T> + Send + 'a> IntoDynBoxed<'a, T> for I {
    fn into_dyn_boxed(self) -> BoxedLIter<'a, T> {
        Box::new(self)
    }
}
