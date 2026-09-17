<div align="center">

<img src="assets/icons/128x128/apps/mesh.png" alt="Mesh Logo" width="100" />

**Mesh**

**A simple and lightweight 3D mesh viewer written in Rust.**

[![Release](https://img.shields.io/github/v/release/odevsa/mesh?label=Release&style=flat-square&color=blue&logo=github)](https://github.com/odevsa/mesh/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-blue?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Build Linux](https://img.shields.io/github/actions/workflow/status/odevsa/mesh/release-linux.yml?label=Linux&style=flat-square)](https://github.com/odevsa/mesh/releases/latest)
[![Build Mac](https://img.shields.io/badge/Mac-unavailable-lightgrey?style=flat-square)](https://github.com/odevsa/mesh/releases/latest)
[![Build Windows](https://img.shields.io/github/actions/workflow/status/odevsa/mesh/release-windows.yml?label=Windows&style=flat-square)](https://github.com/odevsa/mesh/releases/latest)

[Features](#features) •
[Screenshots](#screenshots) •
[Formats](#formats) •
[Installation](#installation) •
[Building](#building) •
[Contributing](#contributing)

<br/>

<img src="assets/screenshot-1.png" alt="Mesh Screenshot" width="85%" />

</div>

## Overview

**Mesh** is a simple desktop viewer designed to quickly inspect 3D files. It focuses on being lightweight and straightforward to use, making it easy to preview models and 3D prints without having to launch heavier software.

## Features

- **Lightweight & Fast**: Built with Rust for quick startup and low memory usage.
- **Multiple Formats**: Supports `.stl`, `.3mf`, `.obj`, and `.gltf` / `.glb` files.
- **Intuitive Camera**: Orbit and zoom around models with smooth mouse and keyboard controls.
- **Customizable Appearance**: Adjust background, mesh colors, lighting, and shadow settings.
- **Reference Grid & Axes**: Configurable ground grid and coordinate axes to help with orientation.
- **Remembers Settings**: Your visual preferences are automatically saved between sessions.
- **Portable**: Single standalone binary with no installation needed.

## Screenshots

<div align="center">

<img src="assets/screenshot-1.png" width="45%" alt="Model View" />
<img src="assets/screenshot-2.png" width="45%" alt="Settings Menu" />

</div>

## Formats

| Format            |    Extension    | Description                                          |
| :---------------- | :-------------: | :--------------------------------------------------- |
| **STL**           |     `.stl`      | Standard Stereolithography format (Binary and ASCII) |
| **3MF**           |     `.3mf`      | 3D Manufacturing Format used in modern 3D printing   |
| **Wavefront OBJ** |     `.obj`      | Classic 3D geometry format                           |
| **glTF / GLB**    | `.gltf`, `.glb` | GL Transmission Format (standard and binary)         |

## Installation

Mesh is completely portable and requires **no installation**:

Download the latest version for your platform from the **[Releases](https://github.com/odevsa/mesh/releases)** page:

### Linux

Choose your preferred format:

- **AppImage** (universal, recommended):
  ```bash
  chmod +x mesh-*-x86_64.AppImage
  ./mesh-*-x86_64.AppImage
  ```
- **Debian / Ubuntu** (`.deb`):
  ```bash
  sudo dpkg -i mesh_*_amd64.deb
  ```
- **Tarball** (`.tar.xz`): Extract and run the `mesh` binary directly.

### Mac

- Comming soon

### Windows

- Download `mesh-*-x86_64-windows.exe` and double-click to run!

## Usage

### From GUI

Simply launch `Mesh` and **double-click** anywhere or **right-click** and select **"Load 3D Model"**.

| Action                      | Shortcut / Gesture                                         |
| :-------------------------- | :--------------------------------------------------------- |
| **Orbit Camera (Mouse)**    | `Left Click` + Drag (with smooth momentum damping)         |
| **Orbit Vertical (Keys)**   | `↑` / `↓` (Arrow Up / Down)                                |
| **Orbit Horizontal (Keys)** | `←` / `→` (Arrow Left / Right)                             |
| **Zoom In / Out**           | `Mouse Wheel` or `+` / `-` keys (speed & dir customizable) |
| **Context Menu**            | `Right Click` anywhere                                     |
| **Open File Dialog**        | `Double Click (LMB)` or select in Context Menu             |
| **Close Menu / Exit**       | `Esc`                                                      |

### From Command Line

You can pass the path to any supported 3D model directly:

```bash
# Linux and Mac
./mesh /path/to/my_model.stl

# Windows
mesh.exe C:\models\sample.3mf
```

## Building

### Prerequisites

- latest stable Rust toolchain

### Compilation

```bash
# 1. Clone the repository
git clone https://github.com/odevsa/mesh.git
cd mesh

# 2. Build in release mode
cargo build --release

# 3. Run the compiled binary
./target/release/mesh [model-path]
```

## Contributing

Contributions, bug reports, and suggestions are welcome!

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Feel free to open an [Issue](https://github.com/odevsa/mesh/issues) if you find a bug or have an idea for a new feature.
