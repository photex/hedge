//!
//! An index based half-edge mesh implementation.
//!

use std::fmt;
use std::sync::atomic;
use std::cmp;
use std::cell::{Cell, RefCell, Ref, RefMut};
use std::marker::PhantomData;
use std::hash::{Hash, Hasher};

pub use crate::kernel::*;
pub use crate::function_sets::*;
pub use crate::iterators::*;

pub mod kernel;
pub mod utils;
pub mod function_sets;
pub mod iterators;

pub type Tag = u32;
pub type Offset = u32;
pub type Generation = u32;
pub type Position = [f32; 3];
pub type Normal = [f32; 3];

////////////////////////////////////////////////////////////////////////////////

/// Marker trait for Index types
pub trait ElementIndex {}

/// Marker trait for structs holding element specific data
pub trait ElementData {}

/// An interface for asserting the validity of components and indices of the mesh.
pub trait IsValid {
    fn is_valid(&self) -> bool;
}

pub trait IsActive {
    fn is_active(&self) -> bool;
}

pub trait Taggable {
    fn tag(&self) -> Tag;
    fn set_tag(&self, tag: Tag);
}

pub trait Storable {
    fn generation(&self) -> Generation;
    fn set_generation(&self, generation: Generation);
    fn status(&self) -> ElementStatus;
    fn set_status(&self, status: ElementStatus);
}

/// Our default value for uninitialized or unconnected components in the mesh.
pub const INVALID_COMPONENT_OFFSET: Offset = 0;

/// Type-safe index into kernel storage.
#[derive(Default, Debug, Clone, Eq)]
pub struct Index<T> {
    pub offset: Offset,
    pub generation: Generation,
    _marker: PhantomData<T>,
}

impl<T: Clone> Copy for Index<T> {}

impl<T> Hash for Index<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.offset.hash(state);
        self.generation.hash(state);
    }
}

impl<T> Index<T> {
    pub fn new(offset: Offset) -> Index<T> {
        Index {
            offset,
            generation: 0,
            _marker: PhantomData::default(),
        }
    }

    pub fn with_generation(offset: Offset, generation: Generation) -> Index<T> {
        Index {
            offset,
            generation,
            _marker: PhantomData::default(),
        }
    }
}

impl<T> PartialOrd for Index<T> {
    fn partial_cmp(&self, other: &Index<T>) -> Option<cmp::Ordering> {
        // Only the offset should matter when it comes to ordering
        self.offset.partial_cmp(&other.offset)
    }
}

impl<T> PartialEq for Index<T> {
    fn eq(&self, other: &Index<T>) -> bool {
        self.offset.eq(&other.offset) && self.generation.eq(&other.generation)
    }
}

impl<T> IsValid for Index<T> {
    fn is_valid(&self) -> bool {
        self.offset != INVALID_COMPONENT_OFFSET
    }
}

/// Whether or not a cell is current or 'removed'
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum ElementStatus {
    ACTIVE,
    INACTIVE,
}

/// Trait for accessing Mesh element properties.
#[derive(Debug, Clone)]
pub struct MeshElement<D: ElementData + Default> {
    tag: Cell<Tag>,
    generation: Cell<Generation>,
    status: Cell<ElementStatus>,
    data: RefCell<D>,
}

impl<D: ElementData + Default> Default for MeshElement<D> {
    fn default() -> Self {
        MeshElement {
            tag: Cell::new(0),
            generation: Cell::new(1),
            status: Cell::new(ElementStatus::INACTIVE),
            data: RefCell::default()
        }
    }
}

impl<D: ElementData + Default> MeshElement<D> {
    pub fn with_data(data: D) -> Self {
        MeshElement {
            data: RefCell::new(data),
            ..MeshElement::default()
        }
    }

    pub fn data(&self) -> Ref<D> {
        self.data.borrow()
    }

    pub fn data_mut(&self) -> RefMut<D> {
        self.data.borrow_mut()
    }
}

impl<D: ElementData + Default> Storable for MeshElement<D> {
    fn generation(&self) -> Generation {
        self.generation.get()
    }

    fn set_generation(&self, generation: Generation) {
        self.generation.set(generation);
    }

    fn status(&self) -> ElementStatus {
        self.status.get()
    }

    fn set_status(&self, status: ElementStatus) {
        self.status.set(status);
    }
}

impl<D: ElementData + Default> Taggable for MeshElement<D> {
    fn tag(&self) -> Tag {
        self.tag.get()
    }

    fn set_tag(&self, tag: Tag) {
        self.tag.set(tag);
    }
}

impl<D: ElementData + Default> IsActive for MeshElement<D> {
    fn is_active(&self) -> bool {
        self.status.get() == ElementStatus::ACTIVE
    }
}

/// TODO: Documentation
#[derive(Debug, Clone, Default)]
pub struct EdgeData {
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
pub type Edge = MeshElement<EdgeData>;
pub type EdgeIndex = Index<Edge>;
impl ElementData for EdgeData {}
impl ElementIndex for  EdgeIndex {}
impl Edge {
    /// Returns true when this edge has a previous and next edge.
    pub fn is_connected(&self) -> bool {
        let data = self.data.borrow();
        data.next_index.is_valid() && data.prev_index.is_valid()
    }
}
impl IsValid for Edge {
    /// An Edge is valid when it has a valid twin index, a valid vertex index
    /// and `is_connected`
    fn is_valid(&self) -> bool {
        let data = self.data.borrow();
        self.is_active() &&
            data.vertex_index.is_valid() &&
            data.twin_index.is_valid() &&
            data.next_index.is_valid() &&
            data.prev_index.is_valid()
    }
}

/// TODO: Documentation
#[derive(Debug, Clone, Default)]
pub struct VertexData {
    /// Index of the outgoing edge
    pub edge_index: EdgeIndex,
    /// Index of point this vertex belongs to
    pub point_index: PointIndex,
}
pub type Vertex = MeshElement<VertexData>;
pub type VertexIndex = Index<Vertex>;
impl ElementData for VertexData {}
impl ElementIndex for VertexIndex {}
impl Vertex {
    pub fn new(edge_index: EdgeIndex, point_index: PointIndex) -> Self {
        Vertex::with_data(VertexData { edge_index, point_index })
    }

    pub fn for_edge(edge_index: EdgeIndex) -> Self {
        Vertex::with_data(VertexData {
            edge_index,
            ..VertexData::default()
        })
    }

    pub fn at_point(point_index: PointIndex) -> Self {
        Vertex::with_data(VertexData {
            point_index,
            ..VertexData::default()
        })
    }
}
impl IsValid for Vertex {
    /// A vertex is considered "valid" as long as it has a valid edge index.
    fn is_valid(&self) -> bool {
        self.is_active() && self.data().edge_index.is_valid()
    }
}

/// TODO: Documentation
#[derive(Debug, Clone, Default)]
pub struct FaceData {
    /// The "root" of an edge loop that defines this face.
    pub edge_index: EdgeIndex,
}
pub type Face = MeshElement<FaceData>;
pub type FaceIndex = Index<Face>;
impl ElementData for FaceData {}
impl ElementIndex for FaceIndex {}
impl Face {
    pub fn new(edge_index: EdgeIndex) -> Self {
        Face::with_data(FaceData { edge_index })
    }
}
impl IsValid for Face {
    /// A face is considered "valid" as long as it has an edge index
    /// other than `INVALID_COMPONENT_INDEX`
    fn is_valid(&self) -> bool {
        self.is_active() && self.data().edge_index.is_valid()
    }
}

#[derive(Debug, Clone)]
pub struct PointData {
    pub position: Position,
}
impl PointData {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        PointData {
            position: [x, y, z],
        }
    }
}
impl Default for PointData {
    fn default() -> Self {
        PointData {
            position: [0.0; 3],
        }
    }
}
pub type Point = MeshElement<PointData>;
pub type PointIndex = Index<Point>;
impl ElementData for PointData {}
impl ElementIndex for PointIndex {}
impl Point {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Point::with_data(PointData::new(x, y, z))
    }
}
impl IsValid for Point {
    fn is_valid(&self) -> bool {
        self.is_active()
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Interface for adding elements to a `Mesh`.
pub trait AddElement<E> {
    fn add_element(&mut self, element: E) -> Index<E>;
}

/// Interface for removing elements to a `Mesh`.
pub trait RemoveElement<E> {
    fn remove_element(&mut self, index: Index<E>);
}

/// Interface for getting an element reference.
pub trait GetElement<E> {
    fn get_element(&self, index: &Index<E>) -> Option<&E>;
}

pub struct Mesh {
    kernel: Kernel,
    tag: atomic::AtomicU32,
}

impl fmt::Debug for Mesh {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Mesh {{ {} points, {} vertices, {} edges, {} faces }}",
            self.point_count(),
            self.vertex_count(),
            self.edge_count(),
            self.face_count()
        )
    }
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh {
            kernel: Kernel::default(),
            tag: atomic::AtomicU32::new(1),
        }
    }

    fn next_tag(&self) -> Tag {
        self.tag.fetch_add(1, atomic::Ordering::SeqCst)
    }

    /// Returns a `FaceFn` for the given index.
    pub fn face(&self, index: FaceIndex) -> FaceFn {
        FaceFn::new(index, &self)
    }

    pub fn face_count(&self) -> usize {
        self.kernel.face_buffer.len() - 1
    }

    pub fn faces(&self) -> impl Iterator<Item=FaceFn> {
        self.kernel.face_buffer.active_cells()
            .map(move |(offset, _)| {
                FaceFn::new(FaceIndex::new(offset as u32), self)
            })
    }

    /// Returns an `EdgeFn` for the given index.
    pub fn edge(&self, index: EdgeIndex) -> EdgeFn {
        EdgeFn::new(index, &self)
    }

    pub fn edge_count(&self) -> usize {
        self.kernel.edge_buffer.len() - 1
    }

    pub fn edges(&self) -> impl Iterator<Item=EdgeFn> {
        self.kernel.edge_buffer.active_cells()
            .map(move |(offset, _)| {
                EdgeFn::new(EdgeIndex::new(offset as u32), self)
            })
    }

    /// Returns a `VertexFn` for the given index.
    pub fn vertex(&self, index: VertexIndex) -> VertexFn {
        VertexFn::new(index, &self)
    }

    pub fn vertex_count(&self) -> usize {
        self.kernel.vertex_buffer.len() - 1
    }

    pub fn vertices(&self) -> impl Iterator<Item=VertexFn> {
        self.kernel.vertex_buffer.active_cells()
            .map(move |(offset, _)| {
                VertexFn::new(VertexIndex::new(offset as u32), self)
            })
    }

    pub fn point_count(&self) -> usize {
        self.kernel.point_buffer.len() - 1
    }

    pub fn add_element<E>(&mut self, element: E) -> Index<E>
        where kernel::Kernel: AddElement<E>
    {
        self.kernel.add_element(element)
    }

    pub fn remove_element<E>(&mut self, index: Index<E>)
        where kernel::Kernel: RemoveElement<E>
    {
        self.kernel.remove_element(index)
    }

    pub fn get_element<E>(&self, index: &Index<E>) -> Option<&E>
        where kernel::Kernel: GetElement<E>
    {
        self.kernel.get_element(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::*;

    #[test]
    fn basic_debug_printing() {
        let _ = env_logger::try_init();

        let edge = Edge::default();
        debug!("{:?}", edge);

        let vertex = Vertex::default();
        debug!("{:?}", vertex);

        let face = Face::default();
        debug!("{:?}", face);

        let point = Point::default();
        debug!("{:?}", point);

        let mesh = Mesh::new();
        debug!("{:?}", mesh);
    }

    #[test]
    fn index_types_are_invalid_by_default() {
        let vert = EdgeIndex::default();
        assert!(!vert.is_valid());

        let edge = EdgeIndex::default();
        assert!(!edge.is_valid());

        let point = PointIndex::default();
        assert!(!point.is_valid());

        let face = FaceIndex::default();
        assert!(!face.is_valid());
    }

    #[test]
    fn default_edge_is_invalid() {
        let edge = Edge::default();
        assert_eq!(edge.is_valid(), false);
    }

    #[test]
    fn default_vertex_is_invalid() {
        let vertex = Vertex::default();
        assert_eq!(vertex.is_valid(), false);
    }

    #[test]
    fn default_face_is_invalid() {
        let face = Face::default();
        assert_eq!(face.is_valid(), false);
    }

    #[test]
    fn default_point_is_invalid() {
        let point = Point::default();
        assert_eq!(point.is_valid(), false);
    }

    #[test]
    fn default_point_is_valid_after_added_to_mesh() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();

        let pindex = {
            let point = Point::default();
            assert_eq!(point.is_valid(), false);
            mesh.add_element(point)
        };

        assert_eq!(mesh.get_element(&pindex).is_some(), true);
    }

    #[test]
    fn initial_mesh_has_default_elements() {
        let _ = env_logger::try_init();
        let mesh = Mesh::new();

        assert_eq!(mesh.edge_count(), 0);
        assert_eq!(mesh.get_element(&EdgeIndex::new(0)).is_some(), false);
        assert_eq!(mesh.kernel.edge_buffer.len(), 1);

        assert_eq!(mesh.face_count(), 0);
        assert_eq!(mesh.get_element(&FaceIndex::new(0)).is_some(), false);
        assert_eq!(mesh.kernel.face_buffer.len(), 1);

        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.get_element(&VertexIndex::new(0)).is_some(), false);
        assert_eq!(mesh.kernel.vertex_buffer.len(), 1);

        assert_eq!(mesh.point_count(), 0);
        assert_eq!(mesh.get_element(&PointIndex::new(0)).is_some(), false);
        assert_eq!(mesh.kernel.point_buffer.len(), 1);
    }

    #[test]
    fn can_add_and_remove_vertices() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();
        let v0 = mesh.add_element(Vertex::default());
        assert_eq!(mesh.vertex_count(), 1);
        assert_eq!(mesh.kernel.vertex_buffer.len(), 2);
        mesh.remove_element(v0);
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.kernel.vertex_buffer.len(), 1);
    }

    #[test]
    fn can_add_and_remove_edges() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();
        let e0 = mesh.add_element(Edge::default());
        assert_eq!(mesh.edge_count(), 1);
        assert_eq!(mesh.kernel.edge_buffer.len(), 2);
        mesh.remove_element(e0);
        assert_eq!(mesh.edge_count(), 0);
        assert_eq!(mesh.kernel.edge_buffer.len(), 1);
    }

    #[test]
    fn can_add_and_remove_faces() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();
        let f0 = mesh.add_element(Face::default());
        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.kernel.face_buffer.len(), 2);
        mesh.remove_element(f0);
        assert_eq!(mesh.face_count(), 0);
        assert_eq!(mesh.kernel.face_buffer.len(), 1);
    }

    #[test]
    fn can_add_and_remove_points() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();
        let p0 = mesh.add_element(Point::default());
        assert_eq!(mesh.point_count(), 1);
        assert_eq!(mesh.kernel.point_buffer.len(), 2);
        mesh.remove_element(p0);
        assert_eq!(mesh.point_count(), 0);
        assert_eq!(mesh.kernel.point_buffer.len(), 1);
    }

    #[test]
    fn can_build_a_simple_mesh_manually() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();

        let p0 = mesh.add_element(Point::new(-1.0, 0.0, 0.0));
        let p1 = mesh.add_element(Point::new(1.0, 0.0, 0.0));
        let p2 = mesh.add_element(Point::new(0.0, 1.0, 0.0));

        let v0 = mesh.add_element(Vertex::at_point(p0));
        let v1 = mesh.add_element(Vertex::at_point(p1));
        let v2 = mesh.add_element(Vertex::at_point(p2));

        let e0 = utils::build_full_edge(&mut mesh, v0, v1);
        let e1 = utils::build_full_edge_from(&mut mesh, e0, v2);
        let e2 = utils::close_edge_loop(&mut mesh, e1, e0);

        let f0 = mesh.add_element(Face::default());
        utils::assign_face_to_loop(&mesh, e0, f0);

        assert!(mesh.edge(e0).is_boundary());
        assert!(mesh.edge(e1).is_boundary());
        assert!(mesh.edge(e2).is_boundary());
        assert_eq!(mesh.edge(e0).face().index, f0);
        assert_eq!(mesh.edge(e1).face().index, f0);
        assert_eq!(mesh.edge(e2).face().index, f0);

        assert_eq!(mesh.edge(e0).vertex().index, v0);
        assert_eq!(mesh.edge(e1).vertex().index, v1);
        assert_eq!(mesh.edge(e2).vertex().index, v2);

        assert_eq!(mesh.edge(e0).twin().vertex().index, v1);
        assert_eq!(mesh.edge(e1).twin().vertex().index, v2);
        assert_eq!(mesh.edge(e2).twin().vertex().index, v0);
    }

    #[test]
    fn can_iterate_over_faces() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();

        mesh.add_element(Face::new(EdgeIndex::new(1)));
        mesh.add_element(Face::new(EdgeIndex::new(4)));
        mesh.add_element(Face::new(EdgeIndex::new(7)));

        assert_eq!(mesh.face_count(), 3);

        let mut faces_iterated_over = 0;

        for face in mesh.faces() {
            assert!(face.is_valid());
            faces_iterated_over += 1;
        }

        assert_eq!(faces_iterated_over, mesh.face_count());
    }

    #[test]
    fn can_iterate_over_vertices() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();

        mesh.add_element(Vertex::new(EdgeIndex::new(1), PointIndex::new(1)));
        mesh.add_element(Vertex::new(EdgeIndex::new(1), PointIndex::new(1)));
        mesh.add_element(Vertex::new(EdgeIndex::new(1), PointIndex::new(1)));
        let v = mesh.add_element(Vertex::new(EdgeIndex::new(4), PointIndex::new(1)));
        mesh.remove_element(v);

        let mut vertices_iterated_over = 0;

        for vert in mesh.vertices() {
            assert!(vert.is_valid());
            assert_ne!(vert.edge().index.offset, 4);
            vertices_iterated_over += 1;
        }

        assert_eq!(vertices_iterated_over, mesh.vertex_count());
    }
}
