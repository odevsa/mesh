use crate::loader::MeshData;
use egui_wgpu::wgpu;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _pad0: f32,
    pub light_dir: [f32; 3],
    pub _pad1: f32,
    pub object_color: [f32; 4],
    pub material_mode: u32, // 0 = Material / Textured, 1 = Solid, 2 = Wireframe
    pub has_texture: u32,
    pub _pad2: [u32; 2],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 5.0],
            _pad0: 0.0,
            light_dir: [0.5, 0.5, 1.0],
            _pad1: 0.0,
            object_color: [0.8, 0.8, 0.8, 1.0],
            material_mode: 0,
            has_texture: 0,
            _pad2: [0, 0],
        }
    }
}

pub struct GpuSubMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub texture_bind_group: wgpu::BindGroup,
}

pub struct SceneRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub render_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub default_texture_bind_group: wgpu::BindGroup,
    pub default_sampler: wgpu::Sampler,

    pub color_texture: Option<wgpu::Texture>,
    pub color_view: Option<wgpu::TextureView>,
    pub depth_texture: Option<wgpu::Texture>,
    pub depth_view: Option<wgpu::TextureView>,
    pub egui_texture_id: Option<egui::TextureId>,

    pub width: u32,
    pub height: u32,

    pub gpu_submeshes: Vec<GpuSubMesh>,
    pub dummy_box_mesh: Vec<GpuSubMesh>,

    pub grid_buffer: Option<wgpu::Buffer>,
    pub grid_vertex_count: u32,

    pub axes_buffer: Option<wgpu::Buffer>,
    pub axes_vertex_count: u32,
}

impl SceneRenderer {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh and Line Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 1x1 white fallback texture
        let white_texture = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("1x1 White Texture"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[255, 255, 255, 255],
        );
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let default_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Default Texture Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&white_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&default_sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Pipeline Layout"),
            bind_group_layouts: &[Some(&uniform_bind_group_layout), Some(&texture_bind_group_layout)],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Two-sided rendering
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Self::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Pipeline for Lines (Grid & Axes)
        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[Some(&uniform_bind_group_layout)],
            immediate_size: 0,
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_line"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_line"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Self::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let dummy_box = Self::create_dummy_box(&device, &default_texture_bind_group);

        Self {
            device,
            queue,
            render_pipeline,
            line_pipeline,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            default_texture_bind_group,
            default_sampler,
            color_texture: None,
            color_view: None,
            depth_texture: None,
            depth_view: None,
            egui_texture_id: None,
            width: 0,
            height: 0,
            gpu_submeshes: Vec::new(),
            dummy_box_mesh: dummy_box,
            grid_buffer: None,
            grid_vertex_count: 0,
            axes_buffer: None,
            axes_vertex_count: 0,
        }
    }

    fn create_dummy_box(device: &wgpu::Device, default_tex: &wgpu::BindGroup) -> Vec<GpuSubMesh> {
        let half = 0.25;
        // 6 faces * 4 vertices = 24 vertices
        let raw_verts = [
            // Top (+Z)
            ([-half, -half, half], [0.0, 0.0, 1.0], [0.0, 0.0]),
            ([half, -half, half], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ([half, half, half], [0.0, 0.0, 1.0], [1.0, 1.0]),
            ([-half, half, half], [0.0, 0.0, 1.0], [0.0, 1.0]),
            // Bottom (-Z)
            ([-half, half, -half], [0.0, 0.0, -1.0], [0.0, 0.0]),
            ([half, half, -half], [0.0, 0.0, -1.0], [1.0, 0.0]),
            ([half, -half, -half], [0.0, 0.0, -1.0], [1.0, 1.0]),
            ([-half, -half, -half], [0.0, 0.0, -1.0], [0.0, 1.0]),
            // Front (+Y)
            ([-half, half, -half], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([half, half, -half], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([half, half, half], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ([-half, half, half], [0.0, 1.0, 0.0], [0.0, 1.0]),
            // Back (-Y)
            ([half, -half, -half], [0.0, -1.0, 0.0], [0.0, 0.0]),
            ([-half, -half, -half], [0.0, -1.0, 0.0], [1.0, 0.0]),
            ([-half, -half, half], [0.0, -1.0, 0.0], [1.0, 1.0]),
            ([half, -half, half], [0.0, -1.0, 0.0], [0.0, 1.0]),
            // Right (+X)
            ([half, -half, -half], [1.0, 0.0, 0.0], [0.0, 0.0]),
            ([half, half, -half], [1.0, 0.0, 0.0], [1.0, 0.0]),
            ([half, half, half], [1.0, 0.0, 0.0], [1.0, 1.0]),
            ([half, -half, half], [1.0, 0.0, 0.0], [0.0, 1.0]),
            // Left (-X)
            ([-half, half, -half], [-1.0, 0.0, 0.0], [0.0, 0.0]),
            ([-half, -half, -half], [-1.0, 0.0, 0.0], [1.0, 0.0]),
            ([-half, -half, half], [-1.0, 0.0, 0.0], [1.0, 1.0]),
            ([-half, half, half], [-1.0, 0.0, 0.0], [0.0, 1.0]),
        ];

        let mut vertices = Vec::new();
        for (pos, norm, uv) in raw_verts {
            vertices.push(Vertex {
                position: pos,
                normal: norm,
                uv,
                color: [0.0, 0.0, 0.0, 0.0],
            });
        }

        let mut indices = Vec::new();
        for i in 0..6 {
            let b = (i * 4) as u32;
            indices.push(b);
            indices.push(b + 1);
            indices.push(b + 2);
            indices.push(b);
            indices.push(b + 2);
            indices.push(b + 3);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dummy Box Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dummy Box Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        vec![GpuSubMesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            texture_bind_group: default_tex.clone(),
        }]
    }

    pub fn update_dummy_box(&mut self, scale: f32, z_offset: f32) {
        let half = 0.25 * scale;
        let raw_verts = [
            // Top (+Z)
            ([-half, -half, half + z_offset], [0.0, 0.0, 1.0], [0.0, 0.0]),
            ([half, -half, half + z_offset], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ([half, half, half + z_offset], [0.0, 0.0, 1.0], [1.0, 1.0]),
            ([-half, half, half + z_offset], [0.0, 0.0, 1.0], [0.0, 1.0]),
            // Bottom (-Z)
            ([-half, half, -half + z_offset], [0.0, 0.0, -1.0], [0.0, 0.0]),
            ([half, half, -half + z_offset], [0.0, 0.0, -1.0], [1.0, 0.0]),
            ([half, -half, -half + z_offset], [0.0, 0.0, -1.0], [1.0, 1.0]),
            ([-half, -half, -half + z_offset], [0.0, 0.0, -1.0], [0.0, 1.0]),
            // Front (+Y)
            ([-half, half, -half + z_offset], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([half, half, -half + z_offset], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([half, half, half + z_offset], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ([-half, half, half + z_offset], [0.0, 1.0, 0.0], [0.0, 1.0]),
            // Back (-Y)
            ([half, -half, -half + z_offset], [0.0, -1.0, 0.0], [0.0, 0.0]),
            ([-half, -half, -half + z_offset], [0.0, -1.0, 0.0], [1.0, 0.0]),
            ([-half, -half, half + z_offset], [0.0, -1.0, 0.0], [1.0, 1.0]),
            ([half, -half, half + z_offset], [0.0, -1.0, 0.0], [0.0, 1.0]),
            // Right (+X)
            ([half, -half, -half + z_offset], [1.0, 0.0, 0.0], [0.0, 0.0]),
            ([half, half, -half + z_offset], [1.0, 0.0, 0.0], [1.0, 0.0]),
            ([half, half, half + z_offset], [1.0, 0.0, 0.0], [1.0, 1.0]),
            ([half, -half, half + z_offset], [1.0, 0.0, 0.0], [0.0, 1.0]),
            // Left (-X)
            ([-half, half, -half + z_offset], [-1.0, 0.0, 0.0], [0.0, 0.0]),
            ([-half, -half, -half + z_offset], [-1.0, 0.0, 0.0], [1.0, 0.0]),
            ([-half, -half, half + z_offset], [-1.0, 0.0, 0.0], [1.0, 1.0]),
            ([-half, half, half + z_offset], [-1.0, 0.0, 0.0], [0.0, 1.0]),
        ];

        let mut vertices = Vec::new();
        for (pos, norm, uv) in raw_verts {
            vertices.push(Vertex {
                position: pos,
                normal: norm,
                uv,
                color: [0.0, 0.0, 0.0, 0.0],
            });
        }

        let mut indices = Vec::new();
        for i in 0..6 {
            let b = (i * 4) as u32;
            indices.push(b);
            indices.push(b + 1);
            indices.push(b + 2);
            indices.push(b);
            indices.push(b + 2);
            indices.push(b + 3);
        }

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dummy Box Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dummy Box Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.dummy_box_mesh = vec![GpuSubMesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            texture_bind_group: self.default_texture_bind_group.clone(),
        }];
    }

    pub fn resize(
        &mut self,
        width: u32,
        height: u32,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        if self.width == width && self.height == height && self.color_texture.is_some() {
            return;
        }

        self.width = width;
        self.height = height;

        let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Offscreen Color Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        if let Some(tex_id) = self.egui_texture_id {
            egui_renderer.update_egui_texture_from_wgpu_texture(
                &self.device,
                &color_view,
                wgpu::FilterMode::Linear,
                tex_id,
            );
        } else {
            let tex_id = egui_renderer.register_native_texture(
                &self.device,
                &color_view,
                wgpu::FilterMode::Linear,
            );
            self.egui_texture_id = Some(tex_id);
        }

        self.color_texture = Some(color_texture);
        self.color_view = Some(color_view);
        self.depth_texture = Some(depth_texture);
        self.depth_view = Some(depth_view);
    }
}

fn compute_normals(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 < positions.len() && i1 < positions.len() && i2 < positions.len() {
            let p0 = glam::Vec3::from(positions[i0]);
            let p1 = glam::Vec3::from(positions[i1]);
            let p2 = glam::Vec3::from(positions[i2]);

            let v01 = p1 - p0;
            let v02 = p2 - p0;
            let v12 = p2 - p1;

            let fnorm = v01.cross(v02);
            let fn_len = fnorm.length();
            if fn_len < 1e-6 {
                continue;
            }
            let fn_norm = fnorm / fn_len;

            let l01 = v01.length();
            let l02 = v02.length();
            let l12 = v12.length();

            if l01 > 1e-6 && l02 > 1e-6 && l12 > 1e-6 {
                let a0 = (v01.dot(v02) / (l01 * l02)).clamp(-1.0, 1.0).acos();
                let a1 = ((-v01.dot(v12)) / (l01 * l12)).clamp(-1.0, 1.0).acos();
                let a2 = ((-v02.dot(-v12)) / (l02 * l12)).clamp(-1.0, 1.0).acos();

                let n0 = glam::Vec3::from(normals[i0]) + fn_norm * a0;
                let n1 = glam::Vec3::from(normals[i1]) + fn_norm * a1;
                let n2 = glam::Vec3::from(normals[i2]) + fn_norm * a2;

                normals[i0] = n0.into();
                normals[i1] = n1.into();
                normals[i2] = n2.into();
            }
        }
    }
    for n in &mut normals {
        let v = glam::Vec3::from(*n);
        *n = if v.length_squared() > 1e-6 {
            v.normalize().into()
        } else {
            [0.0, 0.0, 1.0]
        };
    }
    normals
}

impl SceneRenderer {
    pub fn upload_mesh(&mut self, mesh: &MeshData, scale: f32, center_offset: glam::Vec3, model_pos_offset: glam::Vec3) {
        self.gpu_submeshes.clear();

        for sub in &mesh.submeshes {
            if sub.positions.is_empty() || sub.indices.is_empty() {
                continue;
            }

            let has_valid_normals = sub.normals.len() == sub.positions.len()
                && sub.normals.iter().any(|n| n[0] != 0.0 || n[1] != 0.0 || n[2] != 0.0);
            let computed_normals = if !has_valid_normals {
                Some(compute_normals(&sub.positions, &sub.indices))
            } else {
                None
            };

            let mut vertices = Vec::with_capacity(sub.positions.len());
            for i in 0..sub.positions.len() {
                let p = sub.positions[i];
                let centered = (glam::Vec3::new(p[0], p[1], p[2]) - center_offset) * scale + model_pos_offset;
                let norm = if let Some(ref cn) = computed_normals {
                    cn[i]
                } else {
                    sub.normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0])
                };
                let uv = sub.uvs.get(i).copied().unwrap_or([0.0, 0.0]);
                let col = sub.color.unwrap_or([0.0, 0.0, 0.0, 0.0]);

                vertices.push(Vertex {
                    position: [centered.x, centered.y, centered.z],
                    normal: norm,
                    uv,
                    color: col,
                });
            }

            let mut flat_indices = Vec::with_capacity(sub.indices.len() * 3);
            for tri in &sub.indices {
                flat_indices.push(tri[0]);
                flat_indices.push(tri[1]);
                flat_indices.push(tri[2]);
            }

            let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Submesh Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Submesh Index Buffer"),
                contents: bytemuck::cast_slice(&flat_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let texture_bind_group = if let Some(ref tex_data) = sub.texture {
                let wgpu_tex = self.device.create_texture_with_data(
                    &self.queue,
                    &wgpu::TextureDescriptor {
                        label: Some("Submesh Texture"),
                        size: wgpu::Extent3d {
                            width: tex_data.width,
                            height: tex_data.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &tex_data.rgba,
                );
                let view = wgpu_tex.create_view(&wgpu::TextureViewDescriptor::default());
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Submesh Texture Bind Group"),
                    layout: &self.texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                        },
                    ],
                })
            } else {
                self.default_texture_bind_group.clone()
            };

            self.gpu_submeshes.push(GpuSubMesh {
                vertex_buffer,
                index_buffer,
                index_count: flat_indices.len() as u32,
                texture_bind_group,
            });
        }
    }

    pub fn clear_mesh(&mut self) {
        self.gpu_submeshes.clear();
    }

    pub fn update_grid(
        &mut self,
        half_size: f32,
        divisions: u32,
        color: [f32; 4],
        show_axes: [bool; 3],
    ) {
        let divisions = divisions.max(1);
        let step = (half_size * 2.0) / divisions as f32;
        let mut lines = Vec::new();

        for i in 0..=divisions {
            let x = -half_size + i as f32 * step;
            if show_axes[1] && x.abs() < 1e-4 {
                continue;
            }
            lines.push(LineVertex { position: [x, -half_size, 0.0], color });
            lines.push(LineVertex { position: [x, half_size, 0.0], color });
        }

        for j in 0..=divisions {
            let y = -half_size + j as f32 * step;
            if show_axes[0] && y.abs() < 1e-4 {
                continue;
            }
            lines.push(LineVertex { position: [-half_size, y, 0.0], color });
            lines.push(LineVertex { position: [half_size, y, 0.0], color });
        }

        if !lines.is_empty() {
            self.grid_vertex_count = lines.len() as u32;
            self.grid_buffer = Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Grid Vertex Buffer"),
                contents: bytemuck::cast_slice(&lines),
                usage: wgpu::BufferUsages::VERTEX,
            }));
        } else {
            self.grid_buffer = None;
            self.grid_vertex_count = 0;
        }
    }

    pub fn update_axes(
        &mut self,
        half_size: f32,
        show_axes: [bool; 3],
        show_axis_direction: bool,
    ) {
        let mut lines = Vec::new();
        let cone_size = 0.15;

        // X Axis (Red)
        if show_axes[0] {
            let col = [246.0 / 255.0, 54.0 / 255.0, 82.0 / 255.0, 1.0];
            lines.push(LineVertex { position: [-half_size, 0.0, 0.0], color: col });
            lines.push(LineVertex { position: [half_size, 0.0, 0.0], color: col });
            if show_axis_direction {
                lines.push(LineVertex { position: [half_size, 0.0, 0.0], color: col });
                lines.push(LineVertex { position: [half_size - cone_size, cone_size * 0.4, 0.0], color: col });
                lines.push(LineVertex { position: [half_size, 0.0, 0.0], color: col });
                lines.push(LineVertex { position: [half_size - cone_size, -cone_size * 0.4, 0.0], color: col });
            }
        }

        // Y Axis (Green)
        if show_axes[1] {
            let col = [126.0 / 255.0, 194.0 / 255.0, 18.0 / 255.0, 1.0];
            lines.push(LineVertex { position: [0.0, -half_size, 0.0], color: col });
            lines.push(LineVertex { position: [0.0, half_size, 0.0], color: col });
            if show_axis_direction {
                lines.push(LineVertex { position: [0.0, half_size, 0.0], color: col });
                lines.push(LineVertex { position: [cone_size * 0.4, half_size - cone_size, 0.0], color: col });
                lines.push(LineVertex { position: [0.0, half_size, 0.0], color: col });
                lines.push(LineVertex { position: [-cone_size * 0.4, half_size - cone_size, 0.0], color: col });
            }
        }

        // Z Axis (Blue)
        if show_axes[2] {
            let col = [47.0 / 255.0, 131.0 / 255.0, 227.0 / 255.0, 1.0];
            lines.push(LineVertex { position: [0.0, 0.0, -half_size], color: col });
            lines.push(LineVertex { position: [0.0, 0.0, half_size], color: col });
            if show_axis_direction {
                lines.push(LineVertex { position: [0.0, 0.0, half_size], color: col });
                lines.push(LineVertex { position: [cone_size * 0.4, 0.0, half_size - cone_size], color: col });
                lines.push(LineVertex { position: [0.0, 0.0, half_size], color: col });
                lines.push(LineVertex { position: [-cone_size * 0.4, 0.0, half_size - cone_size], color: col });
            }
        }

        if !lines.is_empty() {
            self.axes_vertex_count = lines.len() as u32;
            self.axes_buffer = Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Axes Vertex Buffer"),
                contents: bytemuck::cast_slice(&lines),
                usage: wgpu::BufferUsages::VERTEX,
            }));
        } else {
            self.axes_buffer = None;
            self.axes_vertex_count = 0;
        }
    }

    pub fn render(
        &mut self,
        uniforms: &Uniforms,
        bg_color: [f32; 4],
        show_dummy: bool,
        show_grid: bool,
        show_axes: bool,
    ) {
        let (Some(color_view), Some(depth_view)) = (&self.color_view, &self.depth_view) else {
            return;
        };

        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Scene Command Encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg_color[0] as f64,
                            g: bg_color[1] as f64,
                            b: bg_color[2] as f64,
                            a: bg_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // 1. Draw Model Submeshes or Dummy Box
            let submeshes_to_draw = if !self.gpu_submeshes.is_empty() {
                &self.gpu_submeshes
            } else if show_dummy {
                &self.dummy_box_mesh
            } else {
                &[][..]
            };

            if !submeshes_to_draw.is_empty() {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                for sub in submeshes_to_draw {
                    pass.set_bind_group(1, &sub.texture_bind_group, &[]);
                    pass.set_vertex_buffer(0, sub.vertex_buffer.slice(..));
                    pass.set_index_buffer(sub.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..sub.index_count, 0, 0..1);
                }
            }

            // 2. Draw Grid (depth-tested against model)
            if show_grid {
                if let (Some(buf), count) = (&self.grid_buffer, self.grid_vertex_count) {
                    if count > 0 {
                        pass.set_pipeline(&self.line_pipeline);
                        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                        pass.set_vertex_buffer(0, buf.slice(..));
                        pass.draw(0..count, 0..1);
                    }
                }
            }

            // 3. Draw Axes (depth-tested against model and grid)
            if show_axes {
                if let (Some(buf), count) = (&self.axes_buffer, self.axes_vertex_count) {
                    if count > 0 {
                        pass.set_pipeline(&self.line_pipeline);
                        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                        pass.set_vertex_buffer(0, buf.slice(..));
                        pass.draw(0..count, 0..1);
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_compilation() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }));
        if let Ok(adapter) = adapter {
            if let Ok((device, _)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                let _shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Mesh and Line Shader Test"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
                });
            }
        }
    }
}
