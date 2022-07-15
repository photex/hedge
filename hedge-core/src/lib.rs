use hedge_element_buffer as hbuf;

#[derive(Default)]
pub struct Vertex;

#[derive(Default)]
pub struct Edge;

#[derive(Default)]
pub struct Face;

pub struct Mesh {
    pub edges: hbuf::ElementBuffer<Edge>,
    pub vertices: hbuf::ElementBuffer<Vertex>,
    pub faces: hbuf::ElementBuffer<Face>,
}

impl Default for Mesh {
    fn default() -> Self {
        Mesh::new()
    }
}

impl Mesh {
    pub fn new() -> Self {
        Mesh {
            edges: Default::default(),
            vertices: Default::default(),
            faces: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut mesh = Mesh::new();
    }
}
