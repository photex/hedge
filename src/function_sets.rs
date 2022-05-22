//! Facades over a mesh and component index to enable fluent adjcency traversals.

use super::*;
use super::iterators;
use std::cell::{Ref, RefMut};

pub trait FunctionSet<'mesh, I: ElementIndex + Default, D: ElementData + Default> {
    fn new(index: I, mesh: &'mesh Mesh) -> Self;
    fn element(&self) -> Option<&'mesh MeshElement<D>>;

    fn maybe(index: Option<I>, mesh: &'mesh Mesh) -> Self
        where Self: std::marker::Sized
    {
        if let Some(index) = index {
            Self::new(index, mesh)
        } else {
            Self::new(Default::default(), mesh)
        }
    }

    fn data(&'mesh self) -> Option<Ref<D>> {
        self.element().map(|e| e.data.borrow())
    }

    fn data_mut(&'mesh self) -> Option<RefMut<D>> {
        self.element().map(|e| e.data.borrow_mut())
    }

//    fn props(&'mesh self) -> Option<&'mesh ElementProperties> {
//        self.element().map(|e| e.props.borrow())
//    }
//
//    fn props_mut(&'mesh self) -> Option<&mut ElementProperties> {
//        self.element().map(|e| &mut e.props.borrow_mut())
//    }
}

/// Function set for operations related to the Face struct
#[derive(Debug, Copy, Clone)]
pub struct FaceFn<'mesh> {
    mesh: &'mesh Mesh,
    pub index: FaceIndex,
}

impl<'mesh> FunctionSet<'mesh, FaceIndex, FaceData> for FaceFn<'mesh> {
    fn new(index: FaceIndex, mesh: &'mesh Mesh) -> Self {
        FaceFn {
            mesh,
            index,
        }
    }

    fn element(&self) -> Option<&'mesh Face> {
        self.mesh.get_element(&self.index)
    }
}

impl<'mesh> FaceFn<'mesh> {
    /// Convert this `FaceFn` to an `EdgeFn`.
    pub fn edge(&self) -> EdgeFn<'mesh> {
        let edge_index = self.data().map(|data| data.edge_index);
        EdgeFn::maybe(edge_index, self.mesh)
    }

    pub fn edges(&self) -> FaceEdges<'mesh> {
        FaceEdges::new(self.mesh.next_tag(), *self)
    }

    pub fn vertices(&self) -> FaceVertices<'mesh> {
        FaceVertices::new(self.mesh.next_tag(), *self)
    }
}

impl<'mesh> IsValid for FaceFn<'mesh> {
    fn is_valid(&self) -> bool {
        self.element().is_some()
    }
}

/// Function set for operations related to the Edge struct
#[derive(Debug, Copy, Clone)]
pub struct EdgeFn<'mesh> {
    mesh: &'mesh Mesh,
    pub index: EdgeIndex,
}

impl<'mesh> FunctionSet<'mesh, EdgeIndex, EdgeData> for EdgeFn<'mesh> {
    fn new(index: EdgeIndex, mesh: &'mesh Mesh) -> Self {
        EdgeFn {
            mesh,
            index,
        }
    }

    fn element(&self) -> Option<&'mesh Edge> {
        self.mesh.get_element(&self.index)
    }
}

impl<'mesh> EdgeFn<'mesh> {
    pub fn is_boundary(&self) -> bool {
        !self.face().is_valid() || !self.twin().face().is_valid()
    }

    /// Convert this `EdgeFn` to an `EdgeFn` of it's next edge
    pub fn next(&self) -> EdgeFn<'mesh> {
        let next_index = self.data().map(|data| data.next_index);
        EdgeFn::maybe(next_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `EdgeFn` of it's prev edge
    pub fn prev(&self) -> EdgeFn<'mesh> {
        let prev_index = self.data().map(|data| data.prev_index);
        EdgeFn::maybe(prev_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `EdgeFn` of it's twin edge
    pub fn twin(&self) -> EdgeFn<'mesh> {
        let twin_index = self.data().map(|data| data.twin_index);
        EdgeFn::maybe(twin_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `FaceFn`
    pub fn face(&self) -> FaceFn<'mesh> {
        let face_index = self.data().map(|data| data.face_index);
        FaceFn::maybe(face_index, self.mesh)
    }

    /// Convert this `EdgeFn` to an `VertexFn`
    pub fn vertex(&self) -> VertexFn<'mesh> {
        let vertex_index = self.data().map(|data| data.vertex_index);
        VertexFn::maybe(vertex_index, self.mesh)
    }
}

impl<'mesh> IsValid for EdgeFn<'mesh> {
    fn is_valid(&self) -> bool {
        self.element().is_some()
    }
}

/// Function set for operations related to the Vertex struct
#[derive(Debug, Copy, Clone)]
pub struct VertexFn<'mesh> {
    mesh: &'mesh Mesh,
    pub index: VertexIndex,
}

impl<'mesh> FunctionSet<'mesh, VertexIndex, VertexData> for VertexFn<'mesh> {
    fn new(index: VertexIndex, mesh: &'mesh Mesh) -> Self {
        VertexFn {
            mesh,
            index,
        }
    }

    fn element(&self) -> Option<&'mesh Vertex> {
        self.mesh.get_element(&self.index)
    }
}

impl<'mesh> VertexFn<'mesh> {
    /// Convert this `VertexFn` to an `EdgeFn`
    pub fn edge(&self) -> EdgeFn<'mesh> {
        let edge_index = self.data().map(|data| data.edge_index);
        EdgeFn::maybe(edge_index, self.mesh)
    }

    pub fn edges(&self) -> iterators::VertexCirculator {
        VertexCirculator::new(self.mesh.next_tag(), *self)
    }

    pub fn point(&self) -> Option<&'mesh Point> {
        self.data().and_then(|data| {
            self.mesh.get_element(&data.point_index)
        })
    }
}

impl<'mesh> IsValid for VertexFn<'mesh> {
    fn is_valid(&self) -> bool {
        self.element().is_some()
    }
}
