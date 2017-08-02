
//!
//! An index based half-edge mesh implementation.
//!

// TODO: Result types for error handling?

extern crate cgmath;

use std::fmt;

pub type EdgeList = Vec<Edge>;
pub type VertexList = Vec<Vertex>;
pub type FaceList = Vec<Face>;

/// An interface for asserting the validity of components in the mesh.
pub trait Validation {
    /// A general blanket test for validity
    fn is_valid(&self) -> bool;
}


/// Our default value for uninitialized or unconnected components in the mesh.
pub const INVALID_COMPONENT_INDEX: usize = 0;

/// Type alias for indices into vertex attribute storage
pub type VertexAttributeIndex = usize;

#[derive(Default, Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct VertexIndex(usize);

impl Validation for VertexIndex {
    fn is_valid(&self) -> bool {
        self.0 != INVALID_COMPONENT_INDEX
    }
}

#[derive(Default, Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct EdgeIndex(usize);

impl Validation for EdgeIndex {
    fn is_valid(&self) -> bool {
        self.0 != INVALID_COMPONENT_INDEX
    }
}

#[derive(Default, Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct FaceIndex(usize);

impl Validation for FaceIndex {
    fn is_valid(&self) -> bool {
        self.0 != INVALID_COMPONENT_INDEX
    }
}


/// Represents the point where two edges meet.
#[derive(Default, Debug)]
pub struct Vertex {
    /// Index of the outgoing edge
    pub edge_index: EdgeIndex,
    /// Index of this vertex's attributes
    pub attr_index: VertexAttributeIndex,
}

impl Vertex {
    pub fn new(edge_index: EdgeIndex) -> Vertex {
        Vertex {
            edge_index: edge_index,
            attr_index: INVALID_COMPONENT_INDEX,
        }
    }
}

impl Validation for Vertex {
    /// A vertex is considered "valid" as long as it as an edge index
    /// other than `INVALID_COMPONENT_INDEX`
    fn is_valid(&self) -> bool {
        self.edge_index.is_valid() /*&&
            self.attr_index.is_valid()*/
    }
}


/// The principle component in a half-edge mesh.
#[derive(Default, Debug)]
pub struct Edge {
    /// The adjacent or 'twin' half-edge
    pub twin_index: EdgeIndex,
    /// The index of the next edge in the loop
    pub next_index: EdgeIndex,
    /// The index of the previous edge in the loop
    pub prev_index: EdgeIndex,

    /// The index of the face this edge loop defines
    pub face_index: FaceIndex,

    /// The index of the Vertex for this edge.
    pub vertex_index: VertexIndex,
}

impl Edge {
    /// Returns true when this edge has a previous and next edge.
    pub fn is_connected(&self) -> bool {
        self.next_index.is_valid() && self.prev_index.is_valid()
    }
}

impl Validation for Edge {
    /// An edge is generally considered "valid" as long as it has a
    /// vertex and a face index other than `INVALID_COMPONENT_INDEX`,
    /// and "is connected".
    fn is_valid(&self) -> bool {
        self.vertex_index.is_valid() && self.twin_index.is_valid()
    }
}


/// A face is defined by the looping connectivity of edges.
#[derive(Default, Debug)]
pub struct Face {
    /// The "root" of an edge loop that defines this face.
    pub edge_index: EdgeIndex,
}

impl Face {
    pub fn new(edge_index: EdgeIndex) -> Face {
        Face {
            edge_index
        }
    }
}

impl Validation for Face {
    /// A face is considered "valid" as long as it has an edge index
    /// other than `INVALID_COMPONENT_INDEX`
    fn is_valid(&self) -> bool {
        self.edge_index.is_valid()
    }
}

/// Function set for operations related to the Face struct
#[derive(Debug)]
pub struct FaceFn<'mesh> {
    mesh: &'mesh Mesh,
    face: &'mesh Face,
    pub index: FaceIndex,
}

impl<'mesh> FaceFn<'mesh> {
    pub fn new(index: FaceIndex, mesh: &'mesh Mesh) -> FaceFn {
        FaceFn {
            mesh: mesh,
            face: mesh.face(index),
            index: index,
        }
    }

    /// Convert this `FaceFn` to an `EdgeFn`.
    pub fn edge(self) -> EdgeFn<'mesh> {
        EdgeFn::new(self.face.edge_index, self.mesh)
    }
}

impl<'mesh> Validation for FaceFn<'mesh> {
    fn is_valid(&self) -> bool {
        self.face.is_valid()
    }
}

/// Function set for operations related to the Vertex struct
#[derive(Debug)]
pub struct VertexFn<'mesh> {
    mesh: &'mesh Mesh,
    vertex: &'mesh Vertex,
    pub index: VertexIndex,
}

impl<'mesh> VertexFn<'mesh> {
    pub fn new(index: VertexIndex, mesh: &'mesh Mesh) -> VertexFn {
        VertexFn {
            mesh: mesh,
            vertex: mesh.vertex(index),
            index: index,
        }
    }

    /// Convert this `VertexFn` to an `EdgeFn`
    pub fn edge(self) -> EdgeFn<'mesh> {
        EdgeFn::new(self.vertex.edge_index, self.mesh)
    }
}

impl<'mesh> Validation for VertexFn<'mesh> {
    fn is_valid(&self) -> bool {
        self.vertex.is_valid()
    }
}

/// Function set for operations related to the Edge struct
#[derive(Debug)]
pub struct EdgeFn<'mesh> {
    mesh: &'mesh Mesh,
    edge: &'mesh Edge,
    pub index: EdgeIndex,
}

impl<'mesh> EdgeFn<'mesh> {
    pub fn new(index: EdgeIndex, mesh: &'mesh Mesh) -> EdgeFn {
        EdgeFn {
            mesh: mesh,
            edge: mesh.edge(index),
            index: index,
        }
    }

    /// Convert this `EdgeFn` to an `EdgeFn` of it's next edge
    pub fn next(self) -> EdgeFn<'mesh> {
        EdgeFn::new(self.edge.next_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `EdgeFn` of it's prev edge
    pub fn prev(self) -> EdgeFn<'mesh> {
        EdgeFn::new(self.edge.prev_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `EdgeFn` of it's twin edge
    pub fn twin(self) -> EdgeFn<'mesh> {
        EdgeFn::new(self.edge.twin_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `FaceFn`
    pub fn face(self) -> FaceFn<'mesh> {
        FaceFn::new(self.edge.face_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `VertexFn`
    pub fn vertex(self) -> VertexFn<'mesh> {
        VertexFn::new(self.edge.vertex_index, self.mesh)
    }
}

impl<'mesh> Validation for EdgeFn<'mesh> {
    fn is_valid(&self) -> bool {
        self.edge.is_valid()
    }
}

/// Implements the fundamental storage operations and represents the principle
/// grouping of all components.
pub struct Mesh {
    pub edge_list: EdgeList,
    pub vertex_list: VertexList,
    pub face_list: FaceList,
}

impl fmt::Debug for Mesh {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Half-Edge Mesh {{ {} vertices, {} edges, {} faces }}",
               self.num_vertices(), self.num_edges(), self.num_faces())
    }
}

impl Mesh {
    /// Creates a new Mesh with an initial component added to each Vec.
    ///
    /// The idea behind having a single invalid component at the front of each
    /// Vec comes from the blog http://ourmachinery.com/post/defaulting-to-zero/
    pub fn new() -> Mesh {
        Mesh {
            edge_list: vec! [
                Edge::default()
            ],
            vertex_list: vec! [
                Vertex::default()
            ],
            face_list: vec! [
                Face::default()
            ]
        }
    }

    /// Mark the two edges as adjacent twins.
    ///
    /// In order for this to be valid each edge should be connected in such a way
    /// that the vertex of each is the same as the vertex of the next edge of each.
    ///
    /// So: `A->Next->Vertex == B->Vertex` && `B->Next->Vertex == A->Vertex`
    ///
    /// _In debug builds we assert the provided indices are valid._
    pub fn set_twin_edges(&mut self, e1: EdgeIndex, e2: EdgeIndex) {
        debug_assert!(e1.is_valid());
        debug_assert!(e2.is_valid());
        // TODO: Disabling this for the moment because it would prevent the use
        //       of the `edge_from_twin` method.
        // debug_assert! {
        //     self.edge(e1).vertex_index == self.edge_fn(e2).next().vertex().index;
        // };
        // debug_assert! {
        //     self.edge(e2).vertex_index == self.edge_fn(e1).next().vertex().index
        // };
        if let Some(ref mut edge1) = self.edge_mut(e1) {
            edge1.twin_index = e2;
        }
        if let Some(ref mut edge2) = self.edge_mut(e2) {
            edge2.twin_index = e1;
        }
    }

    /// Connects the two edges as part of an edge loop.
    ///
    /// _In debug builds we assert that neither index is the default index._
    pub fn connect_edges(&mut self, prev: EdgeIndex, next: EdgeIndex) {
        debug_assert!(prev.is_valid());
        debug_assert!(next.is_valid());
        if let Some(ref mut prev_edge) = self.edge_mut(prev) {
            prev_edge.next_index = next;
        }
        if let Some(ref mut next_edge) = self.edge_mut(next) {
            next_edge.prev_index = prev;
        }
    }

    /// Updates all edges in a loop with the specified face index.
    ///
    /// _In debug builds we assert that each index provided is valid._
    pub fn assign_face_to_loop(&mut self, face_index: FaceIndex, edge_index: EdgeIndex) {
        debug_assert!(face_index.is_valid());
        debug_assert!(edge_index.is_valid());
        if let Some(ref mut face) = self.face_mut(face_index) {
            face.edge_index = edge_index;
        }
        let edge_indices: Vec<EdgeIndex> = EdgeLoop::new(edge_index, &self.edge_list).collect();
        for index in edge_indices {
            if let Some(ref mut edge) = self.edge_mut(index) {
                edge.face_index = face_index;
            }
        }
    }

    /// Create a new edge from the specified vertex.
    ///
    /// _In debug builds we assert that the vertex index is not the default index._
    // pub fn edge_from_vertex(&mut self, vert: VertexIndex) -> EdgeIndex {
    //     debug_assert!(vert.is_valid());
    //     let result = self.add_edge(Edge {
    //         twin_index: EdgeIndex::default(),
    //         next_index: EdgeIndex::default(),
    //         prev_index: EdgeIndex::default(),
    //         face_index: FaceIndex::default(),
    //         vertex_index: vert
    //     });
    //     if let Some(vertex) = self.vertex_mut(vert) {
    //         vertex.edge_index = result;
    //     }
    //     return result;
    // }

    /// Create a new edge as a twin of the specified edge
    ///
    /// _In debug builds we assert that the twin index is not the default index
    /// and that the twins next index is not the default index (since we need
    /// that edge to find the correct vertex index)._
    // pub fn edge_from_twin(&mut self, twin: EdgeIndex) -> EdgeIndex {
    //     debug_assert!(twin.is_valid());
    //     debug_assert!(self.edge(twin).next_index.is_valid());
    //     let vert = self.edge_fn(twin).next().vertex().index;
    //     let result = self.edge_from_vertex(vert);
    //     self.set_twin_edges(result, twin);
    //     return result;
    // }

    pub fn is_boundary_edge(&self, eindex: EdgeIndex) -> bool {
        debug_assert!(eindex.is_valid());
        debug_assert!(self.edge(eindex).is_valid());
        debug_assert!(self.edge_fn(eindex).twin().is_valid());
        self.edge_fn(eindex).twin().face().is_valid()
    }

    /// Create a new edge connected to the previous edge specified.
    ///
    /// _In debug builds we assert that the indices specified are valid._
    pub fn extend_edge_loop(&mut self, prev: EdgeIndex, vert: VertexIndex) -> EdgeIndex {
        debug_assert!(prev.is_valid());
        debug_assert!(vert.is_valid());
        let result = {
            debug_assert!(self.edge(prev).twin_index.is_valid());
            let prev_vert = self.edge_fn(prev).twin().vertex().index;
            self.add_edge(prev_vert, vert)
        };
        self.connect_edges(prev, result);
        return result;
    }

    /// Create a new edge, closing an edge loop, using the `prev` and `next` indices provided.
    ///
    /// _In debug builds we assert that all specified indices are valid._
    pub fn close_edge_loop(&mut self, prev: EdgeIndex, next: EdgeIndex) -> EdgeIndex {
        debug_assert!(prev.is_valid());
        debug_assert!(next.is_valid());
        let vindex_a = self.edge_fn(prev).twin().vertex().index;
        let vindex_b = self.edge_fn(next).vertex().index;
        let result = self.add_edge(vindex_a, vindex_b);
        self.connect_edges(prev, result);
        self.connect_edges(result, next);
        return result;
    }

    /// Inserts the provided `Edge` into the mesh and returns it's `EdgeIndex`
    ///
    /// _In debug builds we assert that the result is a valid index and
    /// that the edge was added to the list._
    pub fn insert_edge(&mut self, edge: Edge) -> EdgeIndex {
        let result = EdgeIndex(self.edge_list.len());
        self.edge_list.push(edge);
        return result;
    }

    pub fn add_edge(&mut self, a: VertexIndex, b: VertexIndex) -> EdgeIndex {
        let eindex_a = EdgeIndex(self.edge_list.len());
        let eindex_b = EdgeIndex(eindex_a.0 + 1);

        let edge_a = Edge {
            twin_index: eindex_b,
            next_index: EdgeIndex::default(),
            prev_index: EdgeIndex::default(),
            face_index: FaceIndex::default(),
            vertex_index: a,
        };
        if let Some(ref mut vert) = self.vertex_mut(a) {
            vert.edge_index = eindex_a;
        }

        let edge_b = Edge {
            twin_index: eindex_a,
            next_index: EdgeIndex::default(),
            prev_index: EdgeIndex::default(),
            face_index: FaceIndex::default(),
            vertex_index: b,
        };
        if let Some(ref mut vert) = self.vertex_mut(b) {
            vert.edge_index = eindex_b;
        }

        self.edge_list.push(edge_a);
        self.edge_list.push(edge_b);

        return eindex_a;
    }

    /// Adds the provided `Vertex` to the mesh and returns it's `VertexIndex`
    pub fn add_vertex(&mut self, vert: Vertex) -> VertexIndex {
        let result = VertexIndex(self.vertex_list.len());
        self.vertex_list.push(vert);
        return result;
    }

    /// Adds the provided `Face` to the mesh and returns it's `FaceIndex`
    ///
    /// _In debug builds we assert that the result is a valid index and
    /// that the face was added to the list._
    pub fn add_face(&mut self, face: Face) -> FaceIndex {
        let result = FaceIndex(self.face_list.len());
        self.face_list.push(face);
        return result;
    }

    pub fn remove_vertex(&mut self, index: VertexIndex) {
        // TODO: In order to remove a vertex you need to circulate over
        //       all connected edges and either remove them, or refuse
        //       remove this vertex until those edges are removed first.
        unimplemented!()
    }

    // TODO: dissolve_vertex

    // TODO: Looking over this I am definitely missing a bunch of edge cases if
    //       I don't ensure that the related components are valid.
    pub fn remove_edge(&mut self, index: EdgeIndex) {
        debug_assert!(index.is_valid());
        let removed_edge = self.edge_list.swap_remove(index.0);

        // Update components affected by removal
        if let Some(ref mut next) = self.edge_mut(removed_edge.next_index) {
            next.prev_index = EdgeIndex::default();
        }
        if let Some(ref mut prev) = self.edge_mut(removed_edge.prev_index) {
            prev.next_index = EdgeIndex::default();
        }
        if let Some(ref mut twin) = self.edge_mut(removed_edge.twin_index) {
            twin.twin_index = EdgeIndex::default();
        }
        if let Some(ref mut face) = self.face_mut(removed_edge.face_index) {
            if face.edge_index == index {
                face.edge_index = removed_edge.next_index;
            }
        }
        // updating the vertex can be a little tricky
        // TODO: Any affected vertex needs to be updated.

        // Update components affected by the swap
        let next_index = self.edge(index).next_index;
        if let Some(ref mut next) = self.edge_mut(next_index) {
            next.prev_index = index;
        }
        let prev_index = self.edge(index).prev_index;
        if let Some(ref mut prev) = self.edge_mut(prev_index) {
            prev.next_index = index;
        }
        let twin_index = self.edge(index).twin_index;
        if let Some(ref mut twin) = self.edge_mut(twin_index) {
            twin.twin_index = index;
        }
        let swapped_index = EdgeIndex(self.edge_list.len());
        let face_index = self.edge(index).face_index;
        if let Some(ref mut face) = self.face_mut(face_index) {
            if face.edge_index == swapped_index {
                face.edge_index = index;
            }
        }
        let swapped_vertex_index = self.edge(index).vertex_index;
        if let Some(ref mut vertex) = self.vertex_mut(swapped_vertex_index) {
            if vertex.edge_index == swapped_index {
                vertex.edge_index = index;
            }
        }
    }

    // TODO: dissolve_edge, collapse_edge

    pub fn remove_face(&mut self, index: FaceIndex) {
        debug_assert!(index.is_valid());
        let removed_face = self.face_list.swap_remove(index.0);

        let edges_of_removed: Vec<EdgeIndex> =
            EdgeLoop::new(removed_face.edge_index, &self.edge_list).collect();
        for eindex in edges_of_removed {
            self.edge_mut(eindex).map(|e| e.face_index = FaceIndex::default());
        }

        let edges_of_swapped: Vec<EdgeIndex> = {
            let swapped_face = self.face(index);
            self.edges(swapped_face).collect()
        };
        for eindex in edges_of_swapped {
            self.edge_mut(eindex).map(|e| e.face_index = index);
        }
    }

    // TODO: dissolve_face, collapse_face

    /// Creates a new face and associated edges with the given vertex indices.
    /// Returns the index of the newly added face.
    ///
    /// _In debug builds we assert that all provided indices are valid._
    pub fn add_triangle(&mut self, a: VertexIndex, b: VertexIndex, c: VertexIndex) -> FaceIndex {
        debug_assert!(a.is_valid());
        debug_assert!(b.is_valid());
        debug_assert!(c.is_valid());

        let e1 = self.add_edge(a, b);
        let e2 = self.extend_edge_loop(e1, c);
        let e3 = self.close_edge_loop(e2, e1);

        let result = self.add_face(Face::new(e1));

        self.edge_mut(e1).map(|e| e.face_index = result);
        self.edge_mut(e2).map(|e| e.face_index = result);
        self.edge_mut(e3).map(|e| e.face_index = result);

        return result;
    }

    /// Creates a new face and associated edges with the given a vertex index and a twin edge index.
    /// Returns the index of the newly added face.
    ///
    /// _In debug builds we assert that the all provided indices are valid._
    pub fn add_adjacent_triangle(&mut self, root_edge: EdgeIndex, vindex: VertexIndex) -> FaceIndex {
        debug_assert!(vindex.is_valid());
        debug_assert!(root_edge.is_valid());
        debug_assert!(self.edge_fn(root_edge).twin().is_valid());

        let e1 = self.edge_fn(root_edge).twin().index;
        let e2 = self.extend_edge_loop(e1, vindex);
        let e3 = self.close_edge_loop(e2, e1);

        let result = self.add_face(Face::new(e1));

        self.edge_mut(e1).map(|e| e.face_index = result);
        self.edge_mut(e2).map(|e| e.face_index = result);
        self.edge_mut(e3).map(|e| e.face_index = result);

        return result;
    }

    /// Create a new face given a slice of vertex indices.
    /// Returns the index of the newly added `Face`.
    ///
    /// If the slice is only 3 elements we just call the `add_triangle`
    /// method instead.
    ///
    /// _In debug builds we assert that all vertex indices are valid._
    pub fn add_polygon(&mut self, verts: &[VertexIndex]) -> FaceIndex {
        debug_assert! {
            verts.iter().all(|v| v.is_valid())
        };
        match verts.len() {
            3 => self.add_triangle(verts[0], verts[1], verts[2]),
            // TODO? 4 => self.add_quad(verts[0], verts[1], verts[2]),
            vert_count => {
                let face_index = self.add_face(Face::default());

                let root_edge_index = self.add_edge(verts[0], verts[1]);
                let mut last_edge_index = root_edge_index;
                for i in 2 .. vert_count - 2 {
                    last_edge_index = self.extend_edge_loop(last_edge_index, verts[i]);
                }
                self.close_edge_loop(last_edge_index, root_edge_index);

                self.assign_face_to_loop(face_index, root_edge_index);

                return face_index;
            }
        }
    }

    /// Returns a `Faces` iterator for this mesh.
    ///
    /// ```
    /// let mesh = hedge::Mesh::new();
    /// for index in mesh.faces() {
    ///    let face = mesh.face(index);
    /// }
    /// ```
    pub fn faces(&self) -> Faces {
        Faces::new(self.face_list.len())
    }

    /// Returns an `EdgeLoop` iterator for the edges around the specified face.
    ///
    /// ```
    /// let mesh = hedge::Mesh::new();
    /// for findex in mesh.faces() {
    ///    let face = mesh.face(findex);
    ///    for eindex in mesh.edges(face) {
    ///        let edge = mesh.edge(eindex);
    ///    }
    /// }
    /// ```
    pub fn edges(&self, face: &Face) -> EdgeLoop {
        EdgeLoop::new(face.edge_index, &self.edge_list)
    }

    /// Returns an `EdgeLoopVertices` iterator for the vertices around the specified face.
    ///
    /// ```
    /// let mesh = hedge::Mesh::new();
    /// for findex in mesh.faces() {
    ///    let face = mesh.face(findex);
    ///    for vindex in mesh.vertices(face) {
    ///        let vertex = mesh.vertex(vindex);
    ///    }
    /// }
    /// ```
    pub fn vertices(&self, face: &Face) -> EdgeLoopVertices {
        EdgeLoopVertices::new(face.edge_index, &self.edge_list)
    }

    pub fn edges_around_vertex(&self, vertex: &Vertex) -> EdgesAroundVertex {
        EdgesAroundVertex::new(vertex.edge_index, &self)
    }

    /// Retrieve an immutable reference to the face specified by `index`
    pub fn face(&self, index: FaceIndex) -> &Face {
        if let Some(result) = self.face_list.get(index.0) {
            result
        } else {
            &self.face_list[0]
        }
    }

    /// Returns a `FaceFn` for the given index.
    ///
    /// ```
    /// use hedge::{Mesh, Vertex};
    /// let mut mesh = Mesh::new();
    ///
    /// let v1 = mesh.add_vertex(Vertex::default());
    /// let v2 = mesh.add_vertex(Vertex::default());
    /// let v3 = mesh.add_vertex(Vertex::default());
    ///
    /// let f1 = mesh.add_triangle(v1, v2, v3);
    ///
    /// assert!(mesh.face_fn(f1).edge().next().vertex().index == v2);
    /// ```
    pub fn face_fn(&self, index: FaceIndex) -> FaceFn {
        FaceFn::new(index, &self)
    }

    /// Obtains a mutable reference to the `Face` for the provided index.
    pub fn face_mut(&mut self, index: FaceIndex) -> Option<&mut Face> {
        if index.is_valid() {
            self.face_list.get_mut(index.0)
        } else {
            None
        }
    }

    pub fn edge(&self, index: EdgeIndex) -> &Edge {
        if let Some(result) = self.edge_list.get(index.0) {
            result
        } else {
            &self.edge_list[0]
        }
    }

    /// Returns an `EdgeFn` for the given index.
    pub fn edge_fn(&self, index: EdgeIndex) -> EdgeFn {
        EdgeFn::new(index, &self)
    }

    /// Obtains a mutable reference to the `Edge` for the provided index.
    pub fn edge_mut(&mut self, index: EdgeIndex) -> Option<&mut Edge> {
        if index.is_valid() {
            self.edge_list.get_mut(index.0)
        } else {
            None
        }
    }

    pub fn vertex(&self, index: VertexIndex) -> &Vertex {
        if let Some(result) = self.vertex_list.get(index.0) {
            result
        } else {
            &self.vertex_list[0]
        }
    }

    /// Returns a `VertexFn` for the given index.
    pub fn vertex_fn(&self, index: VertexIndex) -> VertexFn {
        VertexFn::new(index, &self)
    }

    /// Obtains a mutable reference to the `Vertex` for the provided index.
    pub fn vertex_mut(&mut self, index: VertexIndex) -> Option<&mut Vertex> {
        if index.is_valid() {
            self.vertex_list.get_mut(index.0)
        } else {
            None
        }
    }

    pub fn num_vertices(&self) -> usize {
        self.vertex_list.len() - 1
    }

    pub fn num_faces(&self) -> usize {
        self.face_list.len() - 1
    }

    pub fn num_edges(&self) -> usize {
        self.edge_list.len() - 1
    }
}

/// An iterator that walks an edge loop around a face returning each `VertexIndex` in the loop.
// yeah yeah yeah, I know this is copypasta...
pub struct EdgeLoopVertices<'mesh> {
    edge_list: &'mesh EdgeList,
    initial_index: EdgeIndex,
    current_index: EdgeIndex,
}

impl<'mesh> EdgeLoopVertices<'mesh> {
    pub fn new(index: EdgeIndex, edge_list: &'mesh EdgeList) -> EdgeLoopVertices {
        EdgeLoopVertices {
            edge_list: edge_list,
            initial_index: index,
            current_index: EdgeIndex::default(),
        }
    }
}

impl<'mesh> Iterator for EdgeLoopVertices<'mesh> {
    type Item = VertexIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index.is_valid() {
            self.edge_list.get(self.current_index.0)
                .and_then(|last_edge| {
                    self.current_index = last_edge.next_index;
                    if self.current_index == self.initial_index {
                        None
                    } else {
                        self.edge_list.get(self.current_index.0)
                            .map(|e| e.vertex_index)
                    }
                })
        } else {
            self.current_index = self.initial_index;
            self.edge_list.get(self.current_index.0).map(|e| e.vertex_index)
        }
    }
}

/// An iterator that walks an edge loop around a face returning each `EdgeIndex` in the loop.
pub struct EdgeLoop<'mesh> {
    edge_list: &'mesh EdgeList,
    initial_index: EdgeIndex,
    current_index: EdgeIndex,
}

impl<'mesh> EdgeLoop<'mesh> {
    pub fn new(index: EdgeIndex, edge_list: &'mesh EdgeList) -> EdgeLoop {
        EdgeLoop {
            edge_list: edge_list,
            initial_index: index,
            current_index: EdgeIndex::default(),
        }
    }
}

impl<'mesh> Iterator for EdgeLoop<'mesh> {
    type Item = EdgeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index.is_valid() {
            self.edge_list.get(self.current_index.0).and_then(|current_edge| {
                self.current_index = current_edge.next_index;
                if self.current_index == self.initial_index {
                    None
                } else {
                    Some(self.current_index)
                }
            })
        } else {
            self.current_index = self.initial_index;
            Some(self.current_index)
        }
    }
}

pub struct EdgesAroundVertex<'mesh> {
    mesh: &'mesh Mesh,
    last_index: EdgeIndex,
    next_index: EdgeIndex,
}

impl<'mesh> EdgesAroundVertex<'mesh> {
    pub fn new(edge_index: EdgeIndex, mesh: &'mesh Mesh) -> EdgesAroundVertex<'mesh> {
        EdgesAroundVertex {
            mesh: mesh,
            last_index: EdgeIndex::default(),
            next_index: edge_index,
        }
    }
}

impl<'mesh> Iterator for EdgesAroundVertex<'mesh> {
    type Item = EdgeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.last_index = self.next_index;
        if self.last_index.is_valid() {
            self.next_index = self.mesh.edge_fn(self.last_index)
                .prev().twin().index;
            Some(self.last_index)
        } else {
            None
        }
    }
}

/// An iterator that returns the `FaceIndex` of every Face in the mesh.
///
/// Currently this does not iterate using connectivity information but will
/// perhaps do this in the future.
pub struct Faces {
    face_count: usize,
    previous_index: FaceIndex,
}

impl Faces {
    pub fn new(face_count: usize) -> Faces {
        Faces {
            face_count: face_count,
            previous_index: FaceIndex::default(),
        }
    }
}

// TODO: iterate over faces based on connectivity?
impl Iterator for Faces {
    type Item = FaceIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.previous_index = FaceIndex(self.previous_index.0 + 1);
        if self.previous_index.0 >= self.face_count {
            None
        } else {
            Some(self.previous_index)
        }
    }
}


#[cfg(test)]
mod tests;
