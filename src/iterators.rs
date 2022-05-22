//! Iterators for simple or common mesh traversal patterns.

use log::*;
use super::*;


pub struct VertexCirculator<'mesh> {
    tag: Tag,
    vert: VertexFn<'mesh>,
    last_edge: Option<EdgeFn<'mesh>>,
    central_point: PointIndex,
}

impl<'mesh> VertexCirculator<'mesh> {
    pub fn new(tag: Tag, vert: VertexFn<'mesh>) -> Self {
        VertexCirculator {
            tag,
            vert,
            last_edge: None,
            central_point: vert.data()
                .map(|d| d.point_index)
                .unwrap_or(PointIndex::default())
        }
    }
}

impl<'mesh> Iterator for VertexCirculator<'mesh> {
    type Item = EdgeFn<'mesh>;

    fn next(&mut self) -> Option<Self::Item> {
        self.last_edge = if let Some(last_edge) = self.last_edge {
            let next_edge = last_edge.prev().twin();
            next_edge.element().and_then(|e| {
                if e.tag() == self.tag {
                    debug!("Encountered previously tagged edge.");
                    None
                } else {
                    e.set_tag(self.tag);
                    Some(next_edge)
                }
            }).and_then(|next_edge| {
                if next_edge.is_boundary() {
                    warn!("Vertex circulator terminated due to boundary edge.");
                    None
                } else if let Some(pindex) = next_edge.vertex().data().map(|d| d.point_index) {
                    if pindex == self.central_point {
                        Some(next_edge)
                    } else {
                        debug!("Ending iteration because vertex attributes do not match.");
                        None
                    }
                } else {
                    None
                }
            })
        } else {
            let edge = self.vert.edge();
            edge.element().and_then(|e| {
                e.set_tag(self.tag);
                Some(edge)
            })
        };
        self.last_edge
    }
}

pub struct FaceEdges<'mesh> {
    tag: Tag,
    root_edge: EdgeFn<'mesh>,
    last_edge: Option<EdgeFn<'mesh>>,
}

impl<'mesh> FaceEdges<'mesh> {
    pub fn new(tag: Tag, face: FaceFn<'mesh>) -> Self {
        FaceEdges {
            tag,
            root_edge: face.edge(),
            last_edge: None
        }
    }
}

impl<'mesh> Iterator for FaceEdges<'mesh> {
    type Item = EdgeFn<'mesh>;

    fn next(&mut self) -> Option<Self::Item> {
        self.last_edge = if let Some(last_edge) = self.last_edge {
            let next_edge = last_edge.next();
            next_edge.element()
                .and_then(|edge| {
                    if edge.tag() == self.tag {
                        None
                    } else {
                        edge.set_tag(self.tag);
                        Some(next_edge)
                    }
                })
                .and_then(|next_edge| {
                    if next_edge.index == self.root_edge.index {
                        None
                    } else {
                        Some(next_edge)
                    }
                })
        } else {
            Some(self.root_edge)
        };
        self.last_edge
    }
}

pub struct FaceVertices<'mesh> {
    inner_iter: FaceEdges<'mesh>,
}

impl<'mesh> FaceVertices<'mesh> {
    pub fn new(tag: Tag, face: FaceFn<'mesh>) -> Self {
        let inner_iter = FaceEdges {
            tag,
            root_edge: face.edge(),
            last_edge: None
        };
        FaceVertices { inner_iter }
    }
}

impl<'mesh> Iterator for FaceVertices<'mesh> {
    type Item = VertexFn<'mesh>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next().map(|edge| edge.vertex())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn can_iterate_over_edges_of_face() {
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

        let mut iter_count = 0;
        for edge in mesh.face(f0).edges() {
            assert!(iter_count < 3);
            if edge.index == e0 {
                iter_count += 1;
            } else if edge.index == e1 {
                iter_count += 1;
            } else if edge.index == e2 {
                iter_count += 1;
            } else {
                unreachable!();
            }
        }
        assert_eq!(iter_count, 3);
    }

    #[test]
    fn can_iterate_over_vertices_of_face() {
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
        let _e2 = utils::close_edge_loop(&mut mesh, e1, e0);

        let f0 = mesh.add_element(Face::default());
        utils::assign_face_to_loop(&mesh, e0, f0);

        let mut iter_count = 0;
        for vert in mesh.face(f0).vertices() {
            assert!(iter_count < 3);
            if vert.index == v0 {
                iter_count += 1;
            } else if vert.index == v1 {
                iter_count += 1;
            } else if vert.index == v2 {
                iter_count += 1;
            } else {
                unreachable!();
            }
        }
        assert_eq!(iter_count, 3);
    }

    fn build_fan(points: [PointIndex; 5], mesh: &mut Mesh) -> VertexIndex {
        let v0 = mesh.add_element(Vertex::at_point(points[0]));
        let v1 = mesh.add_element(Vertex::at_point(points[1]));
        let v2 = mesh.add_element(Vertex::at_point(points[4]));

        let e0 = utils::build_full_edge(mesh, v0, v1);
        let e1 = utils::build_full_edge_from(mesh, e0, v2);
        let e2 = utils::close_edge_loop(mesh, e1, e0);

        let f0 = mesh.add_element(Face::default());
        utils::assign_face_to_loop(mesh, e0, f0);

        /////////////////////////////////

        let v3 = mesh.add_element(Vertex::at_point(points[1]));
        let _v4 = mesh.add_element(Vertex::at_point(points[2]));
        let v5 = mesh.add_element(Vertex::at_point(points[4]));

        let e3 = mesh.edge(e1).twin().index;
        utils::assoc_vert_edge(mesh, v5, e3);
        let e4 = utils::build_full_edge_from(mesh, e3, v3);
        let e5 = utils::close_edge_loop(mesh, e4, e3);

        let f1 = mesh.add_element(Face::default());
        utils::assign_face_to_loop(mesh, e3, f1);

        /////////////////////////////////

        let v6 = mesh.add_element(Vertex::at_point(points[2]));
        let _v7 = mesh.add_element(Vertex::at_point(points[3]));
        let v8 = mesh.add_element(Vertex::at_point(points[4]));

        let e6 = mesh.edge(e5).twin().index;
        utils::assoc_vert_edge(mesh, v8, e6);
        let e7 = utils::build_full_edge_from(mesh, e6, v6);
        let e8 = utils::close_edge_loop(mesh, e7, e6);

        let f2 = mesh.add_element(Face::default());
        utils::assign_face_to_loop(mesh, e6, f2);

        /////////////////////////////////

        let _v9  = mesh.add_element(Vertex::at_point(points[3]));
        let _v10 = mesh.add_element(Vertex::at_point(points[0]));
        let v11 = mesh.add_element(Vertex::at_point(points[4]));

        let e9 = mesh.edge(e8).twin().index;
        utils::assoc_vert_edge(mesh, v11, e9);
        let e11 = mesh.edge(e2).twin().index;
        utils::assoc_vert_edge(mesh, v0, e11);
        let _e10 = utils::close_edge_loop(mesh, e9, e11);

        let f3 = mesh.add_element(Face::default());
        utils::assign_face_to_loop(mesh, e9, f3);

        return v2;
    }

    #[test]
    fn can_iterate_around_vertex() {
        let _ = env_logger::try_init();
        let mut mesh = Mesh::new();

        let points = [
            mesh.add_element(Point::new(-1.0, 0.0, 0.0)),
            mesh.add_element(Point::new(0.0, -1.0, 0.0)),
            mesh.add_element(Point::new(1.0, 0.0, 0.0)),
            mesh.add_element(Point::new(0.0, 1.0, 0.0)),
            mesh.add_element(Point::new(0.0, 0.0, 0.0)),
        ];

        let root_vert = build_fan(points, &mut mesh);

        let mut iter_count = 0;
        for _edge in mesh.vertex(root_vert).edges() {
            assert!(iter_count < 4);
            iter_count += 1;
        }
        assert_eq!(iter_count, 4);
    }
}
