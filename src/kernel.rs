use log::*;
use std::cmp::Ordering;
use std::fmt;
use std::iter::Enumerate;
use std::slice::Iter;

use super::{
    AddElement, Edge, EdgeData, ElementData, ElementStatus, Face, FaceData, FaceIndex, GetElement,
    Index, IsActive, IsValid, MeshElement, Offset, Point, PointData, RemoveElement, Storable,
    Vertex, VertexData, VertexIndex,
};

/// A pretty simple wrapper over a pair of 'Vec's.
pub struct ElementBuffer<D: ElementData + Default> {
    pub free_cells: Vec<Index<MeshElement<D>>>,
    pub buffer: Vec<MeshElement<D>>,
}

impl<D: ElementData + Default> Default for ElementBuffer<D> {
    fn default() -> Self {
        ElementBuffer {
            free_cells: Vec::new(),
            buffer: vec![Default::default()],
        }
    }
}

impl<D: ElementData + Default> fmt::Debug for ElementBuffer<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ElementBuffer<> {{ {} items }}", self.len())
    }
}

impl<D: ElementData + Default> ElementBuffer<D> {
    /// Returns the number of currently active cells.
    /// The actual number of items allocated by the buffer might
    /// be different.
    pub fn len(&self) -> usize {
        self.buffer.len() - self.free_cells.len()
    }

    pub fn has_inactive_cells(&self) -> bool {
        !self.free_cells.is_empty()
    }

    fn sort(&mut self) {
        self.buffer[1..].sort_by(|a, b| {
            use crate::ElementStatus::*;
            match (a.status(), b.status()) {
                (ACTIVE, INACTIVE) => Ordering::Less,
                (INACTIVE, ACTIVE) => Ordering::Greater,
                (_, _) => Ordering::Equal,
            }
        });
    }

    pub fn enumerate(&self) -> Enumerate<Iter<MeshElement<D>>> {
        let mut it = self.buffer.iter().enumerate();
        let _ = it.next(); // Always skip the first element since we know it's invalid
        return it;
    }

    pub fn active_cells(&self) -> impl Iterator<Item = (usize, &MeshElement<D>)> {
        self.buffer
            .iter()
            .enumerate()
            .filter(|elem| elem.1.is_active())
    }

    pub fn active_elements(&self) -> impl Iterator<Item = &MeshElement<D>> {
        self.buffer.iter().filter(|elem| elem.is_active())
    }

    fn ensure_active_cell(element: &MeshElement<D>) -> Option<&MeshElement<D>> {
        if element.is_active() {
            Some(element)
        } else {
            None
        }
    }

    fn ensure_matching_generation<'mesh>(
        element: &'mesh MeshElement<D>,
        index: &Index<MeshElement<D>>,
    ) -> Option<&'mesh MeshElement<D>> {
        if index.generation > 0 {
            if element.generation() == index.generation {
                Some(element)
            } else {
                None
            }
        } else {
            Some(element)
        }
    }

    pub fn get(&self, index: &Index<MeshElement<D>>) -> Option<&MeshElement<D>> {
        if index.is_valid() {
            self.buffer
                .get(index.offset as usize)
                .and_then(ElementBuffer::ensure_active_cell)
                .and_then(|e| ElementBuffer::ensure_matching_generation(e, index))
        } else {
            None
        }
    }

    pub fn add(&mut self, element: MeshElement<D>) -> Index<MeshElement<D>> {
        if let Some(index) = self.free_cells.pop() {
            let cell = &mut self.buffer[index.offset as usize];
            *cell = element;
            cell.status.set(ElementStatus::ACTIVE);
            cell.generation.set(index.generation);
            index
        } else {
            let index = Index::with_generation(self.buffer.len() as u32, element.generation.get());
            self.buffer.push(element);
            if let Some(element) = self.buffer.get_mut(index.offset as usize) {
                element.status.set(ElementStatus::ACTIVE);
            }
            index
        }
    }

    pub fn remove(&mut self, index: Index<MeshElement<D>>) {
        if let Some(cell) = self.get(&index) {
            let removed_index = {
                let next_gen = cell.generation() + 1;
                if next_gen == u32::max_value() {
                    cell.set_generation(1);
                } else {
                    cell.set_generation(next_gen);
                }
                cell.set_status(ElementStatus::INACTIVE);
                Index::with_generation(index.offset, cell.generation())
            };
            self.free_cells.push(removed_index);
        }
    }

    fn truncate_inactive(&mut self) {
        let total = self.buffer.len();
        let inactive = self.free_cells.len();
        let active = total - inactive;
        self.free_cells.clear();
        self.buffer.truncate(active);
    }

    fn next_swap_pair(&self) -> Option<(Offset, Offset)> {
        let inactive_offset = self.enumerate().find(|e| !e.1.is_active()).map(|e| e.0);
        let active_offset = self
            .enumerate()
            .rev()
            .find(|e| e.1.is_active())
            .map(|e| e.0);
        if let (Some(inactive_offset), Some(active_offset)) = (inactive_offset, active_offset) {
            if active_offset < inactive_offset {
                debug!("Buffer appears to be successfully sorted!");
                // by the time this is true we should have sorted/swapped
                // all elements so that the inactive inactive elements
                // make up the tail of the buffer.
                None
            } else {
                Some((inactive_offset as u32, active_offset as u32))
            }
        } else {
            debug!("No more swap pairs.");
            None
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

/// Storage interface for Mesh types
#[derive(Debug, Default)]
pub struct Kernel {
    pub edge_buffer: ElementBuffer<EdgeData>,
    pub face_buffer: ElementBuffer<FaceData>,
    pub vertex_buffer: ElementBuffer<VertexData>,
    pub point_buffer: ElementBuffer<PointData>,
}

impl Kernel {
    fn defrag_faces(&mut self) {
        if self.face_buffer.has_inactive_cells() {
            self.face_buffer.sort();
            self.face_buffer
                .active_cells()
                .map(|(offset, face)| {
                    (
                        FaceIndex::with_generation(offset as u32, face.generation.get()),
                        face,
                    )
                })
                .filter(|(index, face)| {
                    let root_edge_index = face.data.borrow().edge_index;
                    if let Some(root_edge) = self.edge_buffer.get(&root_edge_index) {
                        let root_face_index = root_edge.data.borrow().face_index;
                        *index != root_face_index
                    } else {
                        warn!(
                            "The root edge of the face at {:?} points to invalid edge.",
                            root_edge_index
                        );
                        false
                    }
                })
                .for_each(|(face_index, face)| {
                    let root_edge_index = face.data.borrow().edge_index;
                    let mut edge_index = root_edge_index;
                    loop {
                        let edge = &self.edge_buffer.buffer[edge_index.offset as usize];

                        let mut data = edge.data.borrow_mut();
                        // prevent an infinite loop for broken meshes
                        if data.face_index == face_index {
                            break;
                        }
                        data.face_index = face_index;

                        edge_index = data.next_index;
                        if edge_index == root_edge_index {
                            break;
                        }
                    }
                });
            self.face_buffer.truncate_inactive();
        }
    }

    fn defrag_verts(&mut self) {
        if self.vertex_buffer.has_inactive_cells() {
            self.vertex_buffer.sort();
            self.vertex_buffer
                .active_cells()
                .map(|(offset, vertex)| {
                    (
                        VertexIndex::with_generation(offset as u32, vertex.generation.get()),
                        vertex,
                    )
                })
                .filter(|(vert_index, vertex)| {
                    let vert_edge_index = vertex.data.borrow().edge_index;
                    if let Some(edge) = self.edge_buffer.get(&vert_edge_index) {
                        *vert_index != edge.data.borrow().vertex_index
                    } else {
                        warn!("Vertex at {:?} has an invalid edge index.", vert_index);
                        false
                    }
                })
                .for_each(|(vertex_index, vertex)| {
                    let e0 = {
                        let eindex = vertex.data.borrow().edge_index;
                        &self.edge_buffer.buffer[eindex.offset as usize]
                    };
                    e0.data.borrow_mut().vertex_index = vertex_index;
                });
            self.vertex_buffer.truncate_inactive();
        }
    }

    fn defrag_edges(&mut self) {
        if self.edge_buffer.has_inactive_cells() {
            // The edge array can't be sorted as easily
            // as faces and vertices because an edge
            // refers to other elements in the same buffer.
            // Our aproach here needs to be incremental and
            // swap the first active cell from the end of the
            // buffer with first inactive cell from the front
            // of the buffer.
            loop {
                if let Some(offsets) = self.edge_buffer.next_swap_pair() {
                    let inactive_offset = offsets.0;
                    let active_offset = offsets.1;

                    self.edge_buffer
                        .buffer
                        .swap(inactive_offset as usize, active_offset as usize);
                    let swapped = &self.edge_buffer.buffer[inactive_offset as usize];
                    let swapped_data = swapped.data();
                    let swapped_index =
                        Index::with_generation(inactive_offset as u32, swapped.generation.get());

                    if let Some(next_edge) = self.edge_buffer.get(&swapped_data.next_index) {
                        next_edge.data_mut().prev_index = swapped_index;
                    }
                    if let Some(prev_edge) = self.edge_buffer.get(&swapped_data.prev_index) {
                        prev_edge.data_mut().next_index = swapped_index;
                    }
                    if let Some(twin_edge) = self.edge_buffer.get(&swapped_data.twin_index) {
                        twin_edge.data_mut().twin_index = swapped_index;
                    }

                    // For faces and vertices we only want to update the
                    // associated edge index when it matched the original
                    // buffer location.
                    // I'm doing this in case the associated root edge
                    // index for these elements is meaningful or important.

                    if let Some(face) = self.face_buffer.get(&swapped_data.face_index) {
                        let mut face_data = face.data_mut();
                        if face_data.edge_index.offset == active_offset {
                            face_data.edge_index = swapped_index;
                        }
                    }
                    if let Some(vertex) = self.vertex_buffer.get(&swapped_data.vertex_index) {
                        let mut vertex_data = vertex.data_mut();
                        if vertex_data.edge_index.offset == active_offset {
                            vertex_data.edge_index = swapped_index;
                        }
                    }
                } else {
                    break;
                }
            }
            self.edge_buffer.truncate_inactive();
        }
    }

    fn defrag_points(&mut self) {
        if self.point_buffer.has_inactive_cells() {
            // The point structure is potentially
            // referenced from multiple vertices and
            // points do not hold any reference to
            // the vertices associated with them.
            // Because of this we have to search for
            // vertices with a reference to the point
            // at its original location.
            // This also means we can't use the more
            // convienient sort approach.
            loop {
                if let Some(offsets) = self.point_buffer.next_swap_pair() {
                    let inactive_offset = offsets.0;
                    let active_offset = offsets.1;

                    self.point_buffer
                        .buffer
                        .swap(inactive_offset as usize, active_offset as usize);
                    let swapped = &self.point_buffer.buffer[inactive_offset as usize];
                    let swapped_index =
                        Index::with_generation(inactive_offset as u32, swapped.generation.get());

                    self.vertex_buffer.buffer[1..]
                        .iter()
                        .filter(|v| v.is_active() && v.data().point_index.offset == active_offset)
                        .for_each(|v| {
                            v.data_mut().point_index = swapped_index;
                        });
                } else {
                    break;
                }
            }
            self.vertex_buffer.truncate_inactive();
        }
    }

    /// Sorts buffers and drops all inactive elements.
    pub fn defrag(&mut self) {
        if self.inactive_element_count() > 0 {
            self.defrag_faces();
            self.defrag_verts();
            self.defrag_points();
            self.defrag_edges();
        }
    }

    pub fn inactive_element_count(&self) -> usize {
        self.face_buffer.free_cells.len()
            + self.edge_buffer.free_cells.len()
            + self.vertex_buffer.free_cells.len()
            + self.point_buffer.free_cells.len()
    }

    pub fn active_element_count(&self) -> usize {
        self.face_buffer.len()
            + self.edge_buffer.len()
            + self.vertex_buffer.len()
            + self.point_buffer.len()
    }
}

impl GetElement<Point> for Kernel {
    fn get_element(&self, index: &Index<Point>) -> Option<&Point> {
        self.point_buffer.get(index)
    }
}

impl GetElement<Vertex> for Kernel {
    fn get_element(&self, index: &Index<Vertex>) -> Option<&Vertex> {
        self.vertex_buffer.get(index)
    }
}

impl GetElement<Edge> for Kernel {
    fn get_element(&self, index: &Index<Edge>) -> Option<&Edge> {
        self.edge_buffer.get(index)
    }
}

impl GetElement<Face> for Kernel {
    fn get_element(&self, index: &Index<Face>) -> Option<&Face> {
        self.face_buffer.get(index)
    }
}

impl AddElement<Point> for Kernel {
    fn add_element(&mut self, element: Point) -> Index<Point> {
        self.point_buffer.add(element)
    }
}

impl AddElement<Vertex> for Kernel {
    fn add_element(&mut self, element: Vertex) -> Index<Vertex> {
        self.vertex_buffer.add(element)
    }
}

impl AddElement<Edge> for Kernel {
    fn add_element(&mut self, element: Edge) -> Index<Edge> {
        self.edge_buffer.add(element)
    }
}

impl AddElement<Face> for Kernel {
    fn add_element(&mut self, element: Face) -> Index<Face> {
        self.face_buffer.add(element)
    }
}

impl RemoveElement<Point> for Kernel {
    fn remove_element(&mut self, index: Index<Point>) {
        self.point_buffer.remove(index)
    }
}

impl RemoveElement<Vertex> for Kernel {
    fn remove_element(&mut self, index: Index<Vertex>) {
        self.vertex_buffer.remove(index)
    }
}

impl RemoveElement<Edge> for Kernel {
    fn remove_element(&mut self, index: Index<Edge>) {
        self.edge_buffer.remove(index)
    }
}

impl RemoveElement<Face> for Kernel {
    fn remove_element(&mut self, index: Index<Face>) {
        self.face_buffer.remove(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EdgeIndex;

    fn new_edge(kernel: &mut Kernel) -> EdgeIndex {
        let e0 = kernel.add_element(Edge::default());
        let e1 = kernel.add_element(Edge::default());
        match (kernel.get_element(&e0), kernel.get_element(&e1)) {
            (Some(edge0), Some(edge1)) => {
                edge0.data.borrow_mut().twin_index = e1;
                edge1.data.borrow_mut().twin_index = e0;
            }
            _ => panic!("Invalid edge indexes specified: {:?}, {:?}", e0, e1),
        }
        e0
    }

    fn make_twin_edge(kernel: &mut Kernel, twin_index: EdgeIndex) -> EdgeIndex {
        let e0 = kernel.add_element(Edge::with_data(EdgeData {
            twin_index,
            ..EdgeData::default()
        }));
        kernel.edge_buffer.buffer[twin_index.offset as usize]
            .data
            .borrow_mut()
            .twin_index = e0;
        e0
    }

    fn get_twin(kernel: &Kernel, edge_index: EdgeIndex) -> EdgeIndex {
        kernel.edge_buffer.buffer[edge_index.offset as usize]
            .data
            .borrow()
            .twin_index
    }

    fn get_next(kernel: &Kernel, edge_index: EdgeIndex) -> EdgeIndex {
        kernel.edge_buffer.buffer[edge_index.offset as usize]
            .data
            .borrow()
            .next_index
    }

    #[allow(dead_code)]
    fn get_prev(kernel: &Kernel, edge_index: EdgeIndex) -> EdgeIndex {
        kernel.edge_buffer.buffer[edge_index.offset as usize]
            .data
            .borrow()
            .prev_index
    }

    fn connect_edges(
        kernel: &mut Kernel,
        prev_index: EdgeIndex,
        next_index: EdgeIndex,
    ) -> VertexIndex {
        let v0 = kernel.add_element(Vertex::default());
        match (
            kernel.get_element(&prev_index),
            kernel.get_element(&next_index),
        ) {
            (Some(prev), Some(next)) => {
                prev.data.borrow_mut().next_index = next_index;
                next.data.borrow_mut().prev_index = prev_index;
                next.data.borrow_mut().vertex_index = v0;
            }
            _ => panic!(
                "Invalid edge indexes specified: {:?}, {:?}",
                prev_index, next_index
            ),
        }
        v0
    }

    fn set_face_to_loop(kernel: &Kernel, root_edge: EdgeIndex, face_index: FaceIndex) {
        let face = kernel.face_buffer.get(&face_index).unwrap();
        face.data.borrow_mut().edge_index = root_edge;
        let mut edge_index = root_edge;
        loop {
            let edge = &kernel.edge_buffer.buffer[edge_index.offset as usize];
            let mut data = edge.data.borrow_mut();
            if data.face_index == face_index {
                break;
            }
            data.face_index = face_index;
            if data.next_index == root_edge {
                break;
            }
            edge_index = data.next_index;
        }
    }

    fn make_face(kernel: &mut Kernel, root_edge: EdgeIndex) -> FaceIndex {
        let face_index = kernel.add_element(Face::with_data(FaceData {
            edge_index: root_edge,
        }));
        set_face_to_loop(kernel, root_edge, face_index);
        face_index
    }

    fn make_triangle(kernel: &mut Kernel) -> FaceIndex {
        let e0 = new_edge(kernel);
        let e1 = new_edge(kernel);
        let e2 = new_edge(kernel);

        let _ = connect_edges(kernel, e0, e1);
        let _ = connect_edges(kernel, e1, e2);
        let _ = connect_edges(kernel, e2, e0);

        make_face(kernel, e0)
    }

    #[test]
    fn defrag_faces() {
        let _ = env_logger::try_init();
        let mut kernel = Kernel::default();

        let f0 = make_triangle(&mut kernel);
        let root_edge = kernel.face_buffer.buffer[f0.offset as usize]
            .data
            .borrow()
            .edge_index;

        let f1 = make_face(&mut kernel, root_edge);
        let f2 = make_face(&mut kernel, root_edge);
        assert_eq!(kernel.face_buffer.len(), 4);
        assert_eq!(f2.offset, 3);
        assert_eq!(f2.generation, 1);

        kernel.remove_element(f0);
        kernel.remove_element(f1);

        assert!(kernel.face_buffer.has_inactive_cells());
        assert_eq!(kernel.face_buffer.len(), 2);
        assert_eq!(kernel.face_buffer.free_cells.len(), 2);

        let root_face_index = kernel.edge_buffer.buffer[root_edge.offset as usize]
            .data
            .borrow()
            .face_index;
        assert_eq!(root_face_index, f2);

        kernel.defrag_faces();
        assert_eq!(kernel.face_buffer.len(), 2);
        assert_eq!(kernel.face_buffer.free_cells.len(), 0);
        assert!(!kernel.face_buffer.has_inactive_cells());
        assert!(kernel.get_element(&f2).is_none());

        let root_face_index = kernel.edge_buffer.buffer[root_edge.offset as usize]
            .data
            .borrow()
            .face_index;
        assert_ne!(root_face_index, f2);
        assert!(kernel.get_element(&root_face_index).is_some());
        let face_edge_index = kernel.face_buffer.buffer[root_face_index.offset as usize]
            .data
            .borrow()
            .edge_index;
        assert_eq!(face_edge_index, root_edge);
    }

    #[test]
    fn defrag_vertices() {
        let _ = env_logger::try_init();
        let mut kernel = Kernel::default();

        let e0 = new_edge(&mut kernel);
        let e1 = new_edge(&mut kernel);
        let e2 = new_edge(&mut kernel);

        let v0_0 = connect_edges(&mut kernel, e0, e1);
        let v0_1 = connect_edges(&mut kernel, e1, e2);
        let v0_2 = connect_edges(&mut kernel, e2, e0);

        let v1_0 = connect_edges(&mut kernel, e0, e1);
        let v1_1 = connect_edges(&mut kernel, e1, e2);
        let v1_2 = connect_edges(&mut kernel, e2, e0);

        let v2_0 = connect_edges(&mut kernel, e0, e1);
        let v2_1 = connect_edges(&mut kernel, e1, e2);
        let v2_2 = connect_edges(&mut kernel, e2, e0);

        assert_eq!(kernel.vertex_buffer.len(), 10);

        kernel.remove_element(v0_0);
        kernel.remove_element(v0_1);
        kernel.remove_element(v0_2);
        kernel.remove_element(v1_0);
        kernel.remove_element(v1_1);
        kernel.remove_element(v1_2);

        assert_eq!(kernel.vertex_buffer.len(), 4);
        assert_eq!(kernel.vertex_buffer.buffer.len(), 10);

        assert!(kernel.vertex_buffer.get(&v2_0).is_some());
        assert!(kernel.vertex_buffer.get(&v2_1).is_some());
        assert!(kernel.vertex_buffer.get(&v2_2).is_some());

        kernel.defrag_verts();
        assert!(kernel.vertex_buffer.get(&v2_0).is_none());
        assert!(kernel.vertex_buffer.get(&v2_1).is_none());
        assert!(kernel.vertex_buffer.get(&v2_2).is_none());
    }

    #[test]
    fn defrag_edges() {
        let _ = env_logger::try_init();
        let mut kernel = Kernel::default();

        let e0 = new_edge(&mut kernel);
        let e1 = new_edge(&mut kernel);
        let e2 = new_edge(&mut kernel);
        let _v0 = connect_edges(&mut kernel, e0, e1);
        let _v1 = connect_edges(&mut kernel, e1, e2);
        let _v2 = connect_edges(&mut kernel, e2, e0);

        let e3 = get_twin(&kernel, e0);
        let e4 = new_edge(&mut kernel);
        let e5 = new_edge(&mut kernel);
        let _v3 = connect_edges(&mut kernel, e3, e4);
        let _v4 = connect_edges(&mut kernel, e4, e5);
        let _v5 = connect_edges(&mut kernel, e5, e3);

        let e6 = get_twin(&kernel, e4);
        let e7 = get_twin(&kernel, e2);
        let e8 = new_edge(&mut kernel);
        let _v6 = connect_edges(&mut kernel, e6, e7);
        let _v7 = connect_edges(&mut kernel, e7, e8);
        let _v8 = connect_edges(&mut kernel, e8, e6);

        let e9 = get_twin(&kernel, e8);
        let e10 = get_twin(&kernel, e1);
        let e11 = get_twin(&kernel, e5);
        let _v9 = connect_edges(&mut kernel, e9, e10);
        let _v10 = connect_edges(&mut kernel, e10, e11);
        let _v11 = connect_edges(&mut kernel, e11, e9);

        let f0 = make_face(&mut kernel, e0);
        let _f1 = make_face(&mut kernel, e3);
        let _f2 = make_face(&mut kernel, e6);
        let _f3 = make_face(&mut kernel, e9);

        assert_eq!(kernel.active_element_count(), 32);
        assert_eq!(kernel.inactive_element_count(), 0);

        let e12 = make_twin_edge(&mut kernel, e3);
        let e13 = make_twin_edge(&mut kernel, e10);
        let e14 = make_twin_edge(&mut kernel, e7);
        let _v12 = connect_edges(&mut kernel, e12, e13);
        let _v13 = connect_edges(&mut kernel, e13, e14);
        let _v14 = connect_edges(&mut kernel, e14, e12);

        set_face_to_loop(&kernel, e12, f0);
        kernel.remove_element(e0);
        kernel.remove_element(e1);
        kernel.remove_element(e2);

        assert_eq!(kernel.active_element_count(), 35);
        assert_eq!(kernel.inactive_element_count(), 3);

        let face0 = &kernel.face_buffer.buffer[f0.offset as usize];
        let f0e0 = face0.data.borrow().edge_index;
        let f0e1 = get_next(&kernel, f0e0);
        let f0e2 = get_next(&kernel, f0e1);
        assert_eq!(f0e0, get_next(&kernel, f0e2));
        assert_eq!(13, f0e0.offset);
        assert_eq!(14, f0e1.offset);
        assert_eq!(15, f0e2.offset);

        kernel.defrag_edges();
        assert_eq!(kernel.active_element_count(), 35);
        assert_eq!(kernel.inactive_element_count(), 0);

        // Because of how the edge defrag is implemented
        // we expect the offsets for the edges of f0
        // to be at the head of the edge buffer again
        // and basically reversed.
        let face0 = &kernel.face_buffer.buffer[f0.offset as usize];
        let f0e0 = face0.data.borrow().edge_index;
        let f0e1 = get_next(&kernel, f0e0);
        let f0e2 = get_next(&kernel, f0e1);
        assert_eq!(f0e0, get_next(&kernel, f0e2));
        assert_eq!(5, f0e0.offset);
        assert_eq!(3, f0e1.offset);
        assert_eq!(1, f0e2.offset);
    }

    #[test]
    fn defrag_points() {
        let _ = env_logger::try_init();
        let mut kernel = Kernel::default();

        let p0 = kernel.add_element(Point::default());
        let p1 = kernel.add_element(Point::default());
        let p2 = kernel.add_element(Point::default());
        let p3 = kernel.add_element(Point::default());

        let v0 = kernel.add_element(Vertex::with_data(VertexData {
            point_index: p1,
            ..VertexData::default()
        }));
        let v1 = kernel.add_element(Vertex::with_data(VertexData {
            point_index: p1,
            ..VertexData::default()
        }));
        let v2 = kernel.add_element(Vertex::with_data(VertexData {
            point_index: p3,
            ..VertexData::default()
        }));
        let v3 = kernel.add_element(Vertex::with_data(VertexData {
            point_index: p3,
            ..VertexData::default()
        }));

        assert_eq!(
            kernel.vertex_buffer.buffer[v0.offset as usize]
                .data()
                .point_index
                .offset,
            2
        );
        assert_eq!(
            kernel.vertex_buffer.buffer[v1.offset as usize]
                .data()
                .point_index
                .offset,
            2
        );
        assert_eq!(
            kernel.vertex_buffer.buffer[v2.offset as usize]
                .data()
                .point_index
                .offset,
            4
        );
        assert_eq!(
            kernel.vertex_buffer.buffer[v3.offset as usize]
                .data()
                .point_index
                .offset,
            4
        );

        kernel.remove_element(p0);
        kernel.remove_element(p2);
        kernel.defrag_points();

        assert_eq!(
            kernel.vertex_buffer.buffer[v0.offset as usize]
                .data()
                .point_index
                .offset,
            2
        );
        assert_eq!(
            kernel.vertex_buffer.buffer[v1.offset as usize]
                .data()
                .point_index
                .offset,
            2
        );
        assert_eq!(
            kernel.vertex_buffer.buffer[v2.offset as usize]
                .data()
                .point_index
                .offset,
            1
        );
        assert_eq!(
            kernel.vertex_buffer.buffer[v3.offset as usize]
                .data()
                .point_index
                .offset,
            1
        );
    }
}
