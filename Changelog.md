# Hedge Changelog

## 0.1.1

Another major restructure and evaluation of how to approach a "safe" mesh library. Still a bit painful to use but higher level modeling operations will still be added.

The primary reason for stepping back and starting over to some degree was to work on some fundamentals that I sort of ignored in the first approaches. For 0.1.1 this means that I started with the core data layout and kernel and focused on having a more complete foundation.  

As a result, the kernel correctly can "defrag" it's buffers (at least as far as what I've tested). If you end up with a lot of inactive cells after building up a mesh, you can use this to collect memory.

## 0.0.10

Large internal reorganization. Most operations that modify the mesh are now implemented as a series of trait impls with simple but legible types to describe the operation and provide arguments.

Example:

    let mut mesh = Mesh::new();

    let v1 = mesh.add(Vertex::default());
    let v2 = mesh.add(Vertex::default());
    let v3 = mesh.add(Vertex::default());

    // Add a new triangle using these verts.
    let f1 = mesh.add(triangle::FromVerts(v1, v2, v3));

    // oops, that was the wrong order!
    mesh.remove(f1);

    let f1 = mesh.add(triangle::FromVerts(v3, v2, v1));

It makes the docs a bit more complicated (so I'll need to put effort into a guide), but I think it pays off when actually using the API. I believe this opens some doors to more high level operators on meshes.

In addition to the API changes, removing edges is now possible.

One last important note is that the behavior working with edges has changed to *always add or remove the twin edge*. So when adding a triangle to an empty mesh you'll have 6 edges instead of 3, and if you remove an edge you'll be actually removing **two** edges. This ensures consistency and keeps edges next to each other in memory which could open the door to certain optimizations in the future.

At the moment the only operations setup are `AddGeometry` and `RemoveGeometry` but there is already a case to be made for an operation such as `DissolveGeometry` which expands on component removal to also remove components affected and ensure a mesh still conforms to some sanity checks.

## 0.0.9

`EdgeIndex`, `FaceIndex`, and `VertexIndex` are now structs instead of type aliases.

## 0.0.8

- Added method `Edge::is_boundary`
- Added method `Edge::is_connected`
- Added method `Mesh::remove_face`
- Added method `Mesh::remove_edge`
- Added *unimplemented* method `Mesh::remove_vertex`
- Added cgmath dependency
- Moved repo to github

## 0.0.7

- Introducing Changelog.md
- Fixed some typos in documentation
- Updated documentation when missing notices about debug assertions
- Added `Validation` implementations for the function set structs
- Added method `Mesh::assign_face_to_loop`
- Added method `Mesh::add_polygon`

## 0.0.6 - 0.0.1

- Core api exploration, iterators, function set api, and basic primitive operations
