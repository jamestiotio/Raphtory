//! Defines the `Vertex`, which represents a vertex in the graph.
//! A vertex is a node in the graph, and can have properties and edges.
//! It can also be used to navigate the graph.
use crate::{
    core::{
        entities::vertices::vertex_ref::VertexRef,
        utils::{errors::GraphError, time::error::ParseTimeError},
        Prop,
    },
    db::{
        api::{
            properties::Properties,
            view::{
                internal::{DynamicGraph, Immutable, IntoDynamic, MaterializedGraph},
                *,
            },
        },
        graph::{
            path::{PathFromGraph, PathFromVertex},
            vertex::VertexView,
            vertices::Vertices,
            views::{
                deletion_graph::GraphWithDeletions, layer_graph::LayeredGraph,
                window_graph::WindowedGraph,
            },
        },
    },
    prelude::Graph,
    python::{
        graph::{
            edge::{PyEdges, PyNestedEdges},
            properties::{PyNestedPropsIterable, PyPropsList},
        },
        types::wrappers::iterators::*,
        utils::{PyInterval, PyTime},
    },
    *,
};
use chrono::NaiveDateTime;
use itertools::Itertools;
use pyo3::{
    exceptions::{PyIndexError, PyKeyError},
    prelude::*,
    pyclass,
    pyclass::CompareOp,
    pymethods, PyAny, PyObject, PyRef, PyRefMut, PyResult, Python,
};
use python::types::repr::{iterator_repr, Repr};
use std::{collections::HashMap, ops::Deref};

/// A vertex (or node) in the graph.
#[pyclass(name = "Vertex", subclass)]
#[derive(Clone)]
pub struct PyVertex {
    vertex: VertexView<DynamicGraph>,
}

impl<G: GraphViewOps + IntoDynamic> From<VertexView<G>> for PyVertex {
    fn from(value: VertexView<G>) -> Self {
        Self {
            vertex: VertexView {
                graph: value.graph.clone().into_dynamic(),
                vertex: value.vertex,
            },
        }
    }
}

/// Converts a python vertex into a rust vertex.
impl From<PyVertex> for VertexRef {
    fn from(value: PyVertex) -> Self {
        value.vertex.into()
    }
}

/// Defines the `Vertex`, which represents a vertex in the graph.
/// A vertex is a node in the graph, and can have properties and edges.
/// It can also be used to navigate the graph.
#[pymethods]
impl PyVertex {
    /// Rich Comparison for Vertex objects
    pub fn __richcmp__(&self, other: PyRef<PyVertex>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.vertex.id() == other.id()).into_py(py),
            CompareOp::Ne => (self.vertex.id() != other.id()).into_py(py),
            CompareOp::Lt => (self.vertex.id() < other.id()).into_py(py),
            CompareOp::Le => (self.vertex.id() <= other.id()).into_py(py),
            CompareOp::Gt => (self.vertex.id() > other.id()).into_py(py),
            CompareOp::Ge => (self.vertex.id() >= other.id()).into_py(py),
        }
    }

    /// TODO: uncomment when we update to py03 0.2
    /// checks if a vertex is equal to another by their id (ids are unqiue)
    ///
    /// Arguments:
    ///    other: The other vertex to compare to.
    ///
    /// Returns:
    ///   True if the vertices are equal, false otherwise.
    // pub fn __eq__(&self, other: &PyVertex) -> bool {
    //     self.vertex.id() == other.vertex.id()
    // }

    /// Returns the hash of the vertex.
    ///
    /// Returns:
    ///   The vertex id.
    pub fn __hash__(&self) -> u64 {
        self.vertex.id()
    }

    /// Returns the id of the vertex.
    /// This is a unique identifier for the vertex.
    ///
    /// Returns:
    ///    The id of the vertex as an integer.
    #[getter]
    pub fn id(&self) -> u64 {
        self.vertex.id()
    }

    /// Returns the name of the vertex.
    ///
    /// Returns:
    ///     The name of the vertex as a string.
    #[getter]
    pub fn name(&self) -> String {
        self.vertex.name()
    }

    /// Returns the earliest time that the vertex exists.
    ///
    /// Returns:
    ///     The earliest time that the vertex exists as an integer.
    #[getter]
    pub fn earliest_time(&self) -> Option<i64> {
        self.vertex.earliest_time()
    }

    /// Returns the earliest datetime that the vertex exists.
    ///
    /// Returns:
    ///     The earliest datetime that the vertex exists as an integer.
    #[getter]
    pub fn earliest_date_time(&self) -> Option<NaiveDateTime> {
        let earliest_time = self.vertex.earliest_time()?;
        NaiveDateTime::from_timestamp_millis(earliest_time)
    }

    /// Returns the latest time that the vertex exists.
    ///
    /// Returns:
    ///     The latest time that the vertex exists as an integer.
    #[getter]
    pub fn latest_time(&self) -> Option<i64> {
        self.vertex.latest_time()
    }

    /// Returns the latest datetime that the vertex exists.
    ///
    /// Arguments:
    ///    None
    ///
    /// Returns:
    ///     The latest datetime that the vertex exists as an integer.
    #[getter]
    pub fn latest_date_time(&self) -> Option<NaiveDateTime> {
        let latest_time = self.vertex.latest_time()?;
        NaiveDateTime::from_timestamp_millis(latest_time)
    }

    /// The properties of the vertex
    ///
    /// Returns:
    ///     A list of properties.
    #[getter]
    pub fn properties(&self) -> Properties<VertexView<DynamicGraph>> {
        self.vertex.properties()
    }

    /// Get the degree of this vertex (i.e., the number of edges that are incident to it).
    ///
    /// Returns
    ///     The degree of this vertex.
    pub fn degree(&self) -> usize {
        self.vertex.degree()
    }

    /// Get the in-degree of this vertex (i.e., the number of edges that are incident to it from other vertices).
    ///
    /// Returns:
    ///    The in-degree of this vertex.
    pub fn in_degree(&self) -> usize {
        self.vertex.in_degree()
    }

    /// Get the out-degree of this vertex (i.e., the number of edges that are incident to it from this vertex).
    ///
    /// Returns:
    ///   The out-degree of this vertex.
    pub fn out_degree(&self) -> usize {
        self.vertex.out_degree()
    }

    /// Get the edges that are pointing to or from this vertex.
    ///
    /// Returns:
    ///     A list of `Edge` objects.
    #[getter]
    pub fn edges(&self) -> PyEdges {
        let vertex = self.vertex.clone();
        (move || vertex.edges()).into()
    }

    /// Get the edges that are pointing to this vertex.
    ///
    /// Returns:
    ///     A list of `Edge` objects.
    #[getter]
    pub fn in_edges(&self) -> PyEdges {
        let vertex = self.vertex.clone();
        (move || vertex.in_edges()).into()
    }

    /// Get the edges that are pointing from this vertex.
    ///
    /// Returns:
    ///    A list of `Edge` objects.
    #[getter]
    pub fn out_edges(&self) -> PyEdges {
        let vertex = self.vertex.clone();
        (move || vertex.out_edges()).into()
    }

    /// Get the neighbours of this vertex.
    ///
    /// Returns:
    ///
    ///    A list of `Vertex` objects.
    #[getter]
    pub fn neighbours(&self) -> PyPathFromVertex {
        self.vertex.neighbours().into()
    }

    /// Get the neighbours of this vertex that are pointing to it.
    ///
    /// Returns:
    ///   A list of `Vertex` objects.
    #[getter]
    pub fn in_neighbours(&self) -> PyPathFromVertex {
        self.vertex.in_neighbours().into()
    }

    /// Get the neighbours of this vertex that are pointing from it.
    ///
    /// Returns:
    ///   A list of `Vertex` objects.
    #[getter]
    pub fn out_neighbours(&self) -> PyPathFromVertex {
        self.vertex.out_neighbours().into()
    }

    #[doc = default_layer_doc_string!()]
    pub fn default_layer(&self) -> PyVertex {
        self.vertex.default_layer().into()
    }

    #[doc = layers_doc_string!()]
    #[pyo3(signature = (names))]
    pub fn layers(&self, names: Vec<String>) -> Option<VertexView<LayeredGraph<DynamicGraph>>> {
        self.vertex.layer(names)
    }

    #[doc = layers_name_doc_string!()]
    #[pyo3(signature = (name))]
    pub fn layer(&self, name: String) -> Option<VertexView<LayeredGraph<DynamicGraph>>> {
        self.vertex.layer(name)
    }

    /// Returns the history of a vertex, including vertex additions and changes made to vertex.
    ///
    /// Returns:
    ///     A list of timestamps of the event history of vertex.
    pub fn history(&self) -> Vec<i64> {
        self.vertex.history()
    }

    //******  Python  ******//
    pub fn __getitem__(&self, name: &str) -> PyResult<Prop> {
        self.vertex
            .properties()
            .get(name)
            .ok_or(PyKeyError::new_err(format!("Unknown property {}", name)))
    }

    /// Display the vertex as a string.
    pub fn __repr__(&self) -> String {
        self.repr()
    }
}

impl_timeops!(PyVertex, vertex, VertexView<DynamicGraph>, "vertex");

impl Repr for PyVertex {
    fn repr(&self) -> String {
        self.vertex.repr()
    }
}

impl<G: GraphViewOps> Repr for VertexView<G> {
    fn repr(&self) -> String {
        let earliest_time = self.earliest_time().repr();
        let latest_time = self.latest_time().repr();
        let properties: String = self
            .properties()
            .iter()
            .map(|(k, v)| format!("{}: {}", k.deref(), v))
            .join(", ");
        if properties.is_empty() {
            format!(
                "Vertex(name={}, earliest_time={:?}, latest_time={:?})",
                self.name().trim_matches('"'),
                earliest_time,
                latest_time
            )
        } else {
            format!(
                "Vertex(name={}, earliest_time={:?}, latest_time={:?}, properties={})",
                self.name().trim_matches('"'),
                earliest_time,
                latest_time,
                format!("{{{properties}}}")
            )
        }
    }
}

#[pyclass(name = "MutableVertex", extends=PyVertex)]
pub struct PyMutableVertex {
    vertex: VertexView<MaterializedGraph>,
}

impl Repr for PyMutableVertex {
    fn repr(&self) -> String {
        self.vertex.repr()
    }
}

impl From<VertexView<MaterializedGraph>> for PyMutableVertex {
    fn from(vertex: VertexView<MaterializedGraph>) -> Self {
        Self { vertex }
    }
}

impl<G: GraphViewOps + IntoDynamic + Immutable> IntoPy<PyObject> for VertexView<G> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        PyVertex::from(self).into_py(py)
    }
}

impl IntoPy<PyObject> for VertexView<Graph> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        let graph: MaterializedGraph = self.graph.into();
        let vertex = self.vertex;
        let vertex = VertexView { graph, vertex };
        vertex.into_py(py)
    }
}

impl IntoPy<PyObject> for VertexView<GraphWithDeletions> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        let graph: MaterializedGraph = self.graph.into();
        let vertex = self.vertex;
        let vertex = VertexView { graph, vertex };
        vertex.into_py(py)
    }
}

impl IntoPy<PyObject> for VertexView<MaterializedGraph> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        Py::new(
            py,
            (PyMutableVertex::from(self.clone()), PyVertex::from(self)),
        )
        .unwrap() // I think this only fails if we are out of memory? Seems to be unavoidable!
        .into_py(py)
    }
}

#[pymethods]
impl PyMutableVertex {

    /// Add updates to a vertex in the graph at a specified time.
    /// This function allows for the addition of property updates to a vertex within the graph. The updates are time-stamped, meaning they are applied at the specified time.
    ///
    /// Parameters:
    ///     t (PyTime): The timestamp at which the updates should be applied.
    ///     properties (Optional[Dict[str, Prop]]): A dictionary of properties to update.
    ///         Each key is a string representing the property name, and each value is of type Prop representing the property value.
    ///         If None, no properties are updated.
    ///
    /// Returns:
    ///     Result: A result object indicating success or failure. On failure, it contains a GraphError.
    pub fn add_updates(
        &self,
        t: PyTime,
        properties: Option<HashMap<String, Prop>>,
    ) -> Result<(), GraphError> {
        self.vertex.add_updates(t, properties.unwrap_or_default())
    }


    /// Add constant properties to a vertex in the graph.
    /// This function is used to add properties to a vertex that remain constant and do not
    /// change over time. These properties are fundamental attributes of the vertex.
    ///
    /// Parameters:
    ///     properties (Dict[str, Prop]): A dictionary of properties to be added to the vertex.
    ///     Each key is a string representing the property name, and each value is of type Prop
    ///     representing the property value.
    ///
    /// Returns:
    ///     Result: A result object indicating success or failure. On failure, it contains a GraphError..
    pub fn add_constant_properties(&self, properties: HashMap<String, Prop>) -> Result<(), GraphError> {
        self.vertex.add_constant_properties(properties)
    }

    /// Return a string representation of the vertex.
    /// This method provides a human-readable representation of the vertex, which is useful for
    /// debugging and logging purposes.
    ///
    /// Returns:
    ///     str: A string representation of the vertex.
    fn __repr__(&self) -> String {
        self.repr()
    }
}

/// A list of vertices that can be iterated over.
#[pyclass(name = "Vertices")]
pub struct PyVertices {
    pub(crate) vertices: Vertices<DynamicGraph>,
}

impl<G: GraphViewOps + IntoDynamic> From<Vertices<G>> for PyVertices {
    fn from(value: Vertices<G>) -> Self {
        Self {
            vertices: Vertices::new(value.graph.into_dynamic()),
        }
    }
}

impl<G: GraphViewOps + IntoDynamic> IntoPy<PyObject> for Vertices<G> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        PyVertices::from(self).into_py(py)
    }
}

/// Operations on a list of vertices.
/// These use all the same functions as a normal vertex except it returns a list of results.
#[pymethods]
impl PyVertices {
    /// checks if a list of vertices is equal to another list by their idd (ids are unique)
    ///
    /// Arguments:
    ///    other: The other vertices to compare to.
    ///
    /// Returns:
    ///   True if the vertices are equal, false otherwise.
    fn __eq__(&self, other: &PyVertices) -> bool {
        for (v1, v2) in self.vertices.iter().zip(other.vertices.iter()) {
            if v1.id() != v2.id() {
                return false;
            }
        }
        true
    }

    /// Returns an iterator over the vertices ids
    #[getter]
    fn id(&self) -> U64Iterable {
        let vertices = self.vertices.clone();
        (move || vertices.id()).into()
    }

    /// Returns an iterator over the vertices name
    #[getter]
    fn name(&self) -> StringIterable {
        let vertices = self.vertices.clone();
        (move || vertices.name()).into()
    }

    /// Returns an iterator over the vertices earliest time
    #[getter]
    fn earliest_time(&self) -> OptionI64Iterable {
        let vertices = self.vertices.clone();
        (move || vertices.earliest_time()).into()
    }

    /// Returns an iterator over the vertices latest time
    #[getter]
    fn latest_time(&self) -> OptionI64Iterable {
        let vertices = self.vertices.clone();
        (move || vertices.latest_time()).into()
    }

    /// The properties of the vertex
    ///
    /// Returns:
    ///     A List of properties
    #[getter]
    fn properties(&self) -> PyPropsList {
        let vertices = self.vertices.clone();
        (move || vertices.properties()).into()
    }

    /// Returns the number of edges of the vertices
    ///
    /// Returns:
    ///     An iterator of the number of edges of the vertices
    fn degree(&self) -> UsizeIterable {
        let vertices = self.vertices.clone();
        (move || vertices.degree()).into()
    }

    /// Returns the number of in edges of the vertices
    ///
    /// Returns:
    ///     An iterator of the number of in edges of the vertices
    fn in_degree(&self) -> UsizeIterable {
        let vertices = self.vertices.clone();
        (move || vertices.in_degree()).into()
    }

    /// Returns the number of out edges of the vertices
    ///
    /// Returns:
    ///     An iterator of the number of out edges of the vertices
    fn out_degree(&self) -> UsizeIterable {
        let vertices = self.vertices.clone();
        (move || vertices.out_degree()).into()
    }

    /// Returns the edges of the vertices
    ///
    /// Returns:
    ///     An iterator of edges of the vertices
    #[getter]
    fn edges(&self) -> PyNestedEdges {
        let clone = self.vertices.clone();
        (move || clone.edges()).into()
    }

    /// Returns the in edges of the vertices
    ///
    /// Returns:
    ///     An iterator of in edges of the vertices
    #[getter]
    fn in_edges(&self) -> PyNestedEdges {
        let clone = self.vertices.clone();
        (move || clone.in_edges()).into()
    }

    /// Returns the out edges of the vertices
    ///
    /// Returns:
    ///     An iterator of out edges of the vertices
    #[getter]
    fn out_edges(&self) -> PyNestedEdges {
        let clone = self.vertices.clone();
        (move || clone.out_edges()).into()
    }

    /// Get the neighbours of the vertices
    ///
    /// Returns:
    ///     An iterator of the neighbours of the vertices
    #[getter]
    fn neighbours(&self) -> PyPathFromGraph {
        self.vertices.neighbours().into()
    }

    /// Get the in neighbours of the vertices
    ///
    /// Returns:
    ///     An iterator of the in neighbours of the vertices
    #[getter]
    fn in_neighbours(&self) -> PyPathFromGraph {
        self.vertices.in_neighbours().into()
    }

    /// Get the out neighbours of the vertices
    ///
    /// Returns:
    ///     An iterator of the out neighbours of the vertices
    #[getter]
    fn out_neighbours(&self) -> PyPathFromGraph {
        self.vertices.out_neighbours().into()
    }

    /// Collects all vertices into a list
    fn collect(&self) -> Vec<PyVertex> {
        self.__iter__().into_iter().collect()
    }
    #[doc = default_layer_doc_string!()]
    pub fn default_layer(&self) -> PyVertices {
        self.vertices.default_layer().into()
    }

    #[doc = layers_doc_string!()]
    #[pyo3(signature = (name))]
    pub fn layer(&self, name: &str) -> Option<Vertices<LayeredGraph<DynamicGraph>>> {
        self.vertices.layer(name)
    }

    //****** Python *******
    pub fn __iter__(&self) -> PyVertexIterator {
        self.vertices.iter().into()
    }

    pub fn __len__(&self) -> usize {
        self.vertices.len()
    }

    pub fn __bool__(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn __getitem__(&self, vertex: VertexRef) -> PyResult<VertexView<DynamicGraph>> {
        self.vertices
            .get(vertex)
            .ok_or_else(|| PyIndexError::new_err("Vertex does not exist"))
    }

    pub fn __repr__(&self) -> String {
        self.repr()
    }
}

impl_timeops!(PyVertices, vertices, Vertices<DynamicGraph>, "vertices");

impl Repr for PyVertices {
    fn repr(&self) -> String {
        format!("Vertices({})", iterator_repr(self.__iter__().into_iter()))
    }
}

#[pyclass(name = "PathFromGraph")]
pub struct PyPathFromGraph {
    path: PathFromGraph<DynamicGraph>,
}

#[pymethods]
impl PyPathFromGraph {
    fn __iter__(&self) -> PathIterator {
        self.path.iter().into()
    }

    fn collect(&self) -> Vec<Vec<PyVertex>> {
        self.__iter__().into_iter().map(|it| it.collect()).collect()
    }
    #[getter]
    fn id(&self) -> NestedU64Iterable {
        let path = self.path.clone();
        (move || path.id()).into()
    }

    #[getter]
    fn name(&self) -> NestedStringIterable {
        let path = self.path.clone();
        (move || path.name()).into()
    }

    #[getter]
    fn earliest_time(&self) -> NestedOptionI64Iterable {
        let path = self.path.clone();
        (move || path.earliest_time()).into()
    }

    #[getter]
    fn latest_time(&self) -> NestedOptionI64Iterable {
        let path = self.path.clone();
        (move || path.latest_time()).into()
    }

    #[getter]
    fn properties(&self) -> PyNestedPropsIterable {
        let path = self.path.clone();
        (move || path.properties()).into()
    }

    fn degree(&self) -> NestedUsizeIterable {
        let path = self.path.clone();
        (move || path.degree()).into()
    }

    fn in_degree(&self) -> NestedUsizeIterable {
        let path = self.path.clone();
        (move || path.in_degree()).into()
    }

    fn out_degree(&self) -> NestedUsizeIterable {
        let path = self.path.clone();
        (move || path.out_degree()).into()
    }

    #[getter]
    fn edges(&self) -> PyNestedEdges {
        let clone = self.path.clone();
        (move || clone.edges()).into()
    }

    #[getter]
    fn in_edges(&self) -> PyNestedEdges {
        let clone = self.path.clone();
        (move || clone.in_edges()).into()
    }

    #[getter]
    fn out_edges(&self) -> PyNestedEdges {
        let clone = self.path.clone();
        (move || clone.out_edges()).into()
    }

    #[getter]
    fn out_neighbours(&self) -> Self {
        self.path.out_neighbours().into()
    }

    #[getter]
    fn in_neighbours(&self) -> Self {
        self.path.in_neighbours().into()
    }

    #[getter]
    fn neighbours(&self) -> Self {
        self.path.neighbours().into()
    }

    #[doc = default_layer_doc_string!()]
    pub fn default_layer(&self) -> Self {
        self.path.default_layer().into()
    }

    #[doc = layers_doc_string!()]
    #[pyo3(signature = (name))]
    pub fn layer(&self, name: &str) -> Option<PathFromGraph<LayeredGraph<DynamicGraph>>> {
        self.path.layer(name)
    }

    fn __repr__(&self) -> String {
        self.repr()
    }
}

impl_timeops!(PyPathFromGraph, path, PathFromGraph<DynamicGraph>, "path");

impl Repr for PyPathFromGraph {
    fn repr(&self) -> String {
        format!(
            "PathFromGraph({})",
            iterator_repr(self.__iter__().into_iter())
        )
    }
}

impl<G: GraphViewOps + IntoDynamic> From<PathFromGraph<G>> for PyPathFromGraph {
    fn from(value: PathFromGraph<G>) -> Self {
        Self {
            path: PathFromGraph {
                graph: value.graph.clone().into_dynamic(),
                operations: value.operations,
            },
        }
    }
}

impl<G: GraphViewOps + IntoDynamic> IntoPy<PyObject> for PathFromGraph<G> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        PyPathFromGraph::from(self).into_py(py)
    }
}

#[pyclass(name = "PathFromVertex")]
pub struct PyPathFromVertex {
    path: PathFromVertex<DynamicGraph>,
}

impl<G: GraphViewOps + IntoDynamic> From<PathFromVertex<G>> for PyPathFromVertex {
    fn from(value: PathFromVertex<G>) -> Self {
        Self {
            path: PathFromVertex {
                graph: value.graph.clone().into_dynamic(),
                vertex: value.vertex,
                operations: value.operations,
            },
        }
    }
}

impl<G: GraphViewOps + IntoDynamic> IntoPy<PyObject> for PathFromVertex<G> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        PyPathFromVertex::from(self).into_py(py)
    }
}

#[pymethods]
impl PyPathFromVertex {
    fn __iter__(&self) -> PyVertexIterator {
        self.path.iter().into()
    }

    fn collect(&self) -> Vec<PyVertex> {
        self.__iter__().into_iter().collect()
    }

    #[getter]
    fn id(&self) -> U64Iterable {
        let path = self.path.clone();
        (move || path.id()).into()
    }

    #[getter]
    fn name(&self) -> StringIterable {
        let path = self.path.clone();
        (move || path.name()).into()
    }

    #[getter]
    fn earliest_time(&self) -> OptionI64Iterable {
        let path = self.path.clone();
        (move || path.earliest_time()).into()
    }

    #[getter]
    fn latest_time(&self) -> OptionI64Iterable {
        let path = self.path.clone();
        (move || path.latest_time()).into()
    }

    #[getter]
    fn properties(&self) -> PyPropsList {
        let path = self.path.clone();
        (move || path.properties()).into()
    }

    fn in_degree(&self) -> UsizeIterable {
        let path = self.path.clone();
        (move || path.in_degree()).into()
    }

    fn out_degree(&self) -> UsizeIterable {
        let path = self.path.clone();
        (move || path.out_degree()).into()
    }

    fn degree(&self) -> UsizeIterable {
        let path = self.path.clone();
        (move || path.degree()).into()
    }

    #[getter]
    fn edges(&self) -> PyEdges {
        let path = self.path.clone();
        (move || path.edges()).into()
    }

    #[getter]
    fn in_edges(&self) -> PyEdges {
        let path = self.path.clone();
        (move || path.in_edges()).into()
    }

    #[getter]
    fn out_edges(&self) -> PyEdges {
        let path = self.path.clone();
        (move || path.out_edges()).into()
    }

    #[getter]
    fn out_neighbours(&self) -> Self {
        self.path.out_neighbours().into()
    }

    #[getter]
    fn in_neighbours(&self) -> Self {
        self.path.in_neighbours().into()
    }

    #[getter]
    fn neighbours(&self) -> Self {
        self.path.neighbours().into()
    }

    pub fn default_layer(&self) -> Self {
        self.path.default_layer().into()
    }

    #[doc = layers_doc_string!()]
    #[pyo3(signature = (name))]
    pub fn layer(&self, name: &str) -> Option<PathFromVertex<LayeredGraph<DynamicGraph>>> {
        self.path.layer(name)
    }

    fn __repr__(&self) -> String {
        self.repr()
    }
}

impl_timeops!(PyPathFromVertex, path, PathFromVertex<DynamicGraph>, "path");

impl Repr for PyPathFromVertex {
    fn repr(&self) -> String {
        format!(
            "PathFromVertex({})",
            iterator_repr(self.__iter__().into_iter())
        )
    }
}

#[pyclass(name = "VertexIterator")]
pub struct PyVertexIterator {
    iter: Box<dyn Iterator<Item = PyVertex> + Send>,
}

impl From<Box<dyn Iterator<Item = VertexView<DynamicGraph>> + Send>> for PyVertexIterator {
    fn from(value: Box<dyn Iterator<Item = VertexView<DynamicGraph>> + Send>) -> Self {
        Self {
            iter: Box::new(value.map(|v| v.into())),
        }
    }
}

impl IntoIterator for PyVertexIterator {
    type Item = PyVertex;
    type IntoIter = Box<dyn Iterator<Item = PyVertex> + Send>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter
    }
}

#[pymethods]
impl PyVertexIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyVertex> {
        slf.iter.next()
    }
}

impl From<Box<dyn Iterator<Item = PyVertex> + Send>> for PyVertexIterator {
    fn from(value: Box<dyn Iterator<Item = PyVertex> + Send>) -> Self {
        Self { iter: value }
    }
}

#[pyclass]
pub struct PathIterator {
    pub(crate) iter: Box<dyn Iterator<Item = PyPathFromVertex> + Send>,
}

impl IntoIterator for PathIterator {
    type Item = PyPathFromVertex;
    type IntoIter = Box<dyn Iterator<Item = PyPathFromVertex> + Send>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter
    }
}

impl From<Box<dyn Iterator<Item = PathFromVertex<DynamicGraph>> + Send>> for PathIterator {
    fn from(value: Box<dyn Iterator<Item = PathFromVertex<DynamicGraph>> + Send>) -> Self {
        Self {
            iter: Box::new(value.map(|path| path.into())),
        }
    }
}

#[pymethods]
impl PathIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyPathFromVertex> {
        slf.iter.next()
    }
}

py_iterable!(PyVertexIterable, VertexView<DynamicGraph>, PyVertex);

#[pymethods]
impl PyVertexIterable {
    #[getter]
    fn id(&self) -> U64Iterable {
        let builder = self.builder.clone();
        (move || builder().id()).into()
    }

    #[getter]
    fn name(&self) -> StringIterable {
        let vertices = self.builder.clone();
        (move || vertices().name()).into()
    }

    #[getter]
    fn earliest_time(&self) -> OptionI64Iterable {
        let vertices = self.builder.clone();
        (move || vertices().earliest_time()).into()
    }

    #[getter]
    fn latest_time(&self) -> OptionI64Iterable {
        let vertices = self.builder.clone();
        (move || vertices().latest_time()).into()
    }

    #[getter]
    fn properties(&self) -> PyPropsList {
        let vertices = self.builder.clone();
        (move || vertices().properties()).into()
    }

    fn degree(&self) -> UsizeIterable {
        let vertices = self.builder.clone();
        (move || vertices().degree()).into()
    }

    fn in_degree(&self) -> UsizeIterable {
        let vertices = self.builder.clone();
        (move || vertices().in_degree()).into()
    }

    fn out_degree(&self) -> UsizeIterable {
        let vertices = self.builder.clone();
        (move || vertices().out_degree()).into()
    }

    #[getter]
    fn edges(&self) -> PyEdges {
        let clone = self.builder.clone();
        (move || clone().edges()).into()
    }

    #[getter]
    fn in_edges(&self) -> PyEdges {
        let clone = self.builder.clone();
        (move || clone().in_edges()).into()
    }

    #[getter]
    fn out_edges(&self) -> PyEdges {
        let clone = self.builder.clone();
        (move || clone().out_edges()).into()
    }

    #[getter]
    fn out_neighbours(&self) -> Self {
        let builder = self.builder.clone();
        (move || builder().out_neighbours()).into()
    }

    #[getter]
    fn in_neighbours(&self) -> Self {
        let builder = self.builder.clone();
        (move || builder().in_neighbours()).into()
    }

    #[getter]
    fn neighbours(&self) -> Self {
        let builder = self.builder.clone();
        (move || builder().neighbours()).into()
    }
}

py_nested_iterable!(PyNestedVertexIterable, VertexView<DynamicGraph>);

#[pymethods]
impl PyNestedVertexIterable {
    #[getter]
    fn id(&self) -> NestedU64Iterable {
        let builder = self.builder.clone();
        (move || builder().id()).into()
    }

    #[getter]
    fn name(&self) -> NestedStringIterable {
        let vertices = self.builder.clone();
        (move || vertices().name()).into()
    }

    #[getter]
    fn earliest_time(&self) -> NestedOptionI64Iterable {
        let vertices = self.builder.clone();
        (move || vertices().earliest_time()).into()
    }

    #[getter]
    fn latest_time(&self) -> NestedOptionI64Iterable {
        let vertices = self.builder.clone();
        (move || vertices().latest_time()).into()
    }

    #[getter]
    fn properties(&self) -> PyNestedPropsIterable {
        let vertices = self.builder.clone();
        (move || vertices().properties()).into()
    }

    fn degree(&self) -> NestedUsizeIterable {
        let vertices = self.builder.clone();
        (move || vertices().degree()).into()
    }

    fn in_degree(&self) -> NestedUsizeIterable {
        let vertices = self.builder.clone();
        (move || vertices().in_degree()).into()
    }

    fn out_degree(&self) -> NestedUsizeIterable {
        let vertices = self.builder.clone();
        (move || vertices().out_degree()).into()
    }

    #[getter]
    fn edges(&self) -> PyNestedEdges {
        let clone = self.builder.clone();
        (move || clone().edges()).into()
    }

    #[getter]
    fn in_edges(&self) -> PyNestedEdges {
        let clone = self.builder.clone();
        (move || clone().in_edges()).into()
    }

    #[getter]
    fn out_edges(&self) -> PyNestedEdges {
        let clone = self.builder.clone();
        (move || clone().out_edges()).into()
    }

    #[getter]
    fn out_neighbours(&self) -> Self {
        let builder = self.builder.clone();
        (move || builder().out_neighbours()).into()
    }

    #[getter]
    fn in_neighbours(&self) -> Self {
        let builder = self.builder.clone();
        (move || builder().in_neighbours()).into()
    }

    #[getter]
    fn neighbours(&self) -> Self {
        let builder = self.builder.clone();
        (move || builder().neighbours()).into()
    }
}
