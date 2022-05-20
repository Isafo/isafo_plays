use crate::shader;
use egui::Context;
use glam::{vec3, Mat4, UVec3, Vec2, Vec3};
use std::mem;
use wgpu::util::DeviceExt;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, Debug, AsBytes, FromBytes)]
pub(crate) struct TriUniforms {
    pub transform: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Default, Copy, AsBytes, FromBytes)]
pub(crate) struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

static TRI_VERTEX_DATA: &[Vertex] = &[
    Vertex {
        pos: [0.0, -0.5, 0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        pos: [0.5, 0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        pos: [-0.5, 0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
];

static TRI_INDEX_DATA: &[u16] = &[0, 1, 2];

pub struct App {
    x_pos: f32,

    tri_vertex_buf: wgpu::Buffer,
    tri_index_buf: wgpu::Buffer,

    _bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    shader_storage_buffer: wgpu::Buffer,

    cs_pipeline: wgpu::ComputePipeline,
    //cs_shader_storage_buffer: wgpu::Buffer,
    cs_vertex_buf: wgpu::Buffer,
    cs_index_buf: wgpu::Buffer,
    // cs_vertex_bind_group_layout: wgpu::BindGroupLayout,
    // cs_vertex_bind_group: wgpu::BindGroup,
    // cs_index_bind_group_layout: wgpu::BindGroupLayout,
    // cs_index_bind_group: wgpu::BindGroup,
    scalar_data: wgpu::Texture,
}

impl App {
    pub fn new(
        device: &wgpu::Device,
        surface_format: &wgpu::TextureFormat,
        texture_size: UVec3,
    ) -> App {
        let tri_vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: TRI_VERTEX_DATA.as_bytes(),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let tri_index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: TRI_INDEX_DATA.as_bytes(),
            usage: wgpu::BufferUsages::INDEX,
        });

        let shader_storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: mem::size_of::<TriUniforms>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    shader_storage_buffer.as_entire_buffer_binding(),
                ),
            }],
            label: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let (vs_module, fs_module) = shader::compile(
            device,
            include_str!("shaders/tri.vert"),
            include_str!("shaders/tri.frag"),
        );

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 6 * mem::size_of::<f32>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 3 * mem::size_of::<f32>() as u64,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: *surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            multiview: None,
        });

        let cs_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: (wgpu::BufferBindingType::Storage { read_only: false }),
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: None,
            });

        let cs_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&cs_bind_group_layout],
            push_constant_ranges: &[],
        });

        let cs_module = shader::compile_cs(device, include_str!("shaders/compute_test.comp"));
        let cs_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&cs_pipeline_layout),
            module: &cs_module,
            entry_point: "main",
        });

        let cell_count = texture_size.x * texture_size.y * texture_size.z;
        let max_triangle_count = 4 * cell_count as usize;
        let max_index_count = max_triangle_count * 3;
        let mut vertex_data = vec![Vertex::default(); max_triangle_count];
        let mut index_data = vec![u32::default(); max_index_count];

        let vertex_data_slice_size = vertex_data.len() * std::mem::size_of::<Vertex>();
        let vertex_slice_size = vertex_data_slice_size as wgpu::BufferAddress;

        let cs_vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: vertex_slice_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let index_data_slice_size = index_data.len() * std::mem::size_of::<Vertex>();
        let index_slice_size = index_data_slice_size as wgpu::BufferAddress;

        // let cs_vertex_bind_group_layout = cs_pipeline.get_bind_group_layout(0);
        // let cs_vertex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: None,
        //     layout: &cs_vertex_bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: cs_vertex_buf.as_entire_binding(),
        //     }],
        // });

        let cs_index_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: index_slice_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // let cs_index_bind_group_layout = cs_pipeline.get_bind_group_layout(1);
        // let cs_index_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: None,
        //     layout: &cs_index_bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: cs_index_buf.as_entire_binding(),
        //     }],
        // });

        let scalar_data = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: texture_size.x,
                height: texture_size.y,
                depth_or_array_layers: texture_size.z,
            },
            mip_level_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            sample_count: 1,
        });

        App {
            x_pos: 0.0,
            tri_vertex_buf,
            tri_index_buf,
            shader_storage_buffer,
            _bind_group_layout: bind_group_layout,
            bind_group,
            pipeline,
            cs_pipeline,
            //cs_shader_storage_buffer,
            cs_vertex_buf,
            cs_index_buf,
            // cs_vertex_bind_group_layout,
            // cs_vertex_bind_group,
            // cs_index_bind_group_layout,
            // cs_index_bind_group,
            scalar_data,
        }
    }

    pub fn ui(&mut self, context: &Context) {
        egui::Window::new("Window").show(context, |ui| {
            ui.label("Hello world!");
            ui.add(egui::DragValue::new(&mut self.x_pos).speed(0.1));
        });
    }

    pub fn cs_fun(&mut self, encoder: &mut wgpu::CommandEncoder, texture_size: UVec3) {
        let mut cs_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cs_pass.set_pipeline(&self.cs_pipeline);
        //cs_pass.set_bind_group(0, &self.cs_vertex_bind_group, &[]);
        //cs_pass.set_bind_group(1, &self.cs_index_bind_group, &[]);
        cs_pass.insert_debug_marker("compute density values + mc");
        cs_pass.dispatch(texture_size.x, texture_size.y, texture_size.z);
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // setup uniforms and send to gpu
        let trans = Mat4::from_translation(vec3(self.x_pos, 0.0, 0.0));
        let uniforms = TriUniforms {
            transform: trans.to_cols_array_2d(),
        };

        let temp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: uniforms.as_bytes(),
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.shader_storage_buffer,
            0,
            mem::size_of::<TriUniforms>() as u64,
        );

        // Issue draw call
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_index_buffer(self.tri_index_buf.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.tri_vertex_buf.slice(..));
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw_indexed(0..(TRI_INDEX_DATA.len() as u32), 0, 0..1);
    }
}
