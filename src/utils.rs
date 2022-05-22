use log::*;
use super::*;

/// Given two vertex indices, create an adjacent edge pair
pub fn build_full_edge(
    mesh: &mut Mesh,
    v0: VertexIndex,
    v1: VertexIndex
) -> EdgeIndex {
    let e0 = mesh.add_element(Edge {
        data: RefCell::new(EdgeData {
            vertex_index: v0,
            ..EdgeData::default()
        }),
        ..Edge::default()
    });

    let e1 = mesh.add_element(Edge {
        data: RefCell::new( EdgeData {
            twin_index: e0,
            vertex_index: v1,
            ..EdgeData::default()
        }),
        ..Edge::default()
    });

    mesh.get_element(&e0).map(|e| e.data_mut().twin_index = e1);
    mesh.get_element(&v0).map(|e| e.data_mut().edge_index = e0);
    mesh.get_element(&v1).map(|e| e.data_mut().edge_index = e1);

    return e0;
}

pub fn build_half_edge(
    mesh: &mut Mesh,
    twin: EdgeIndex,
    vert: VertexIndex,
) -> EdgeIndex {
    let e0 = mesh.add_element(
        Edge::with_data(EdgeData {
            vertex_index: vert,
            twin_index: twin,
            ..EdgeData::default()
        })
    );

    mesh.get_element(&twin).map(|e| e.data_mut().twin_index = e0);
    mesh.get_element(&vert).map(|v| v.data_mut().edge_index = e0);

    return e0;
}

pub fn assoc_vert_edge(
    mesh: &Mesh,
    vert: VertexIndex,
    edge: EdgeIndex
) {
    mesh.get_element(&vert).map(|v| v.data_mut().edge_index = edge);
    mesh.get_element(&edge).map(|e| e.data_mut().vertex_index = vert);
}

/// Given an edge index, and a vertex index, creates a new edge connected to the specified edge
pub fn build_full_edge_from(
    mesh: &mut Mesh,
    prev: EdgeIndex,
    v1: VertexIndex
) -> EdgeIndex {
    let e0 = {
        let v0 = mesh.edge(prev).twin().vertex().index;
        build_full_edge(mesh, v0, v1)
    };
    connect_edges(mesh, prev, e0);
    return e0;
}

pub fn close_edge_loop(
    mesh: &mut Mesh,
    prev: EdgeIndex,
    next: EdgeIndex
) -> EdgeIndex {
    let v0 = mesh.edge(prev).twin().element().map(|e| e.data().vertex_index);
    let v1 = mesh.edge(next).element().map(|e| e.data().vertex_index);

    if let (Some(v0), Some(v1)) = (v0, v1) {
        let e0 = build_full_edge(mesh, v0, v1);
        connect_edges(mesh, prev, e0);
        connect_edges(mesh, e0, next);
        e0
    } else {
        error!("Failed to properly discover associated vertices.");
        EdgeIndex::default()
    }
}

/// Associates a previous and next edge
pub fn connect_edges(
    mesh: &mut Mesh,
    prev: EdgeIndex,
    next: EdgeIndex
) {
    mesh.get_element(&prev).map(|e| e.data.borrow_mut().next_index = next);
    mesh.get_element(&next).map(|e| e.data.borrow_mut().prev_index = prev);
}

pub fn assign_face_to_loop(
    mesh: &Mesh,
    root_edge_index: EdgeIndex,
    face_index: FaceIndex
) {
    let face = mesh.face(face_index);
    if let Some(mut data) = face.data_mut() {
        data.edge_index = root_edge_index;
    } else {
        error!("Invalid face index specified: {:?}", face_index);
        return;
    }
    let mut edge = face.edge();
    loop {
        if let Some(mut data) = edge.data_mut() {
            if data.face_index == face.index {
                break;
            }
            data.face_index = face.index;
            if data.next_index == root_edge_index {
                break;
            }
        } else {
            error!("Invalid edge index! {:?}", edge.index);
            break;
        }
        edge = edge.next();
    }
}
