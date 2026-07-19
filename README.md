# Mesh

A lightweight desktop application to view 3D mesh files written in rust.

## Supported formats

- STL
- OBJ
- glTF (gltf/glb)

## Building

1. Build:

   ```bash
   cargo build --release
   ```

2. Run with a file:

   ```bash
   ./target/release/mesh /path/to/model.stl
   ```
