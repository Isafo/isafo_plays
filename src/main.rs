use std::iter;

use crate::app::App;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use glam::{uvec3, UVec3, Vec3};
use wgpu::{util::DeviceExt, Extent3d};
use winit::{event::Event, event_loop::ControlFlow};
mod app;
mod shader;

const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;

#[derive(Default, Copy, Clone)]
struct Vertex {
    pos: Vec3,
    normal: Vec3,
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title("isafo_plays")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::downlevel_defaults(),
            label: None,
        },
        None,
    ))
    .unwrap();

    let size = window.inner_size();
    let surface_format = surface.get_preferred_format(&adapter).unwrap();
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let texture_size = uvec3(size.width, size.height, 64u32);
    let density_data = device.create_texture(&wgpu::TextureDescriptor {
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

    let mut state = egui_winit::State::new(4096, &window);
    let context = egui::Context::default();

    let mut egui_rpass = RenderPass::new(&device, surface_format, 1);

    let mut app = App::new(&device, &surface_format);

    let cs_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    let cs_module = shader::compile_cs(&device, "compute_test.glsl");
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
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

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: vertex_slice_size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let index_data_slice_size = index_data.len() * std::mem::size_of::<Vertex>();
    let index_slice_size = index_data_slice_size as wgpu::BufferAddress;

    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: index_slice_size,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let vertex_bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let vertex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &vertex_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: vertex_buffer.as_entire_binding(),
        }],
    });

    let index_bind_group_layout = compute_pipeline.get_bind_group_layout(1);
    let index_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &index_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: index_buffer.as_entire_binding(),
        }],
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(..) => {
                let mut cs_pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
                cs_pass.set_pipeline(&compute_pipeline);
                cs_pass.set_bind_group(0, &vertex_bind_group, &[]);
                cs_pass.set_bind_group(1, &index_bind_group, &[]);
                cs_pass.insert_debug_marker("compute density values + mc");
                cs_pass.dispatch(texture_size.x, texture_size.y, texture_size.z);

                let output_frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(wgpu::SurfaceError::Outdated) => {
                        // This error occurs when the app is minimized on Windows.
                        // Silently return here to prevent spamming the console with:
                        // "The underlying surface has changed, and therefore the swap chain must be updated"
                        return;
                    }
                    Err(e) => {
                        eprintln!("Dropped frame with error: {}", e);
                        return;
                    }
                };
                let output_view = output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let input = state.take_egui_input(&window);
                context.begin_frame(input);

                app.ui(&context);

                let output = context.end_frame();
                let paint_jobs = context.tessellate(output.shapes);

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

                app.draw(&device, &queue, &output_view, &mut encoder);

                let screen_descriptor = ScreenDescriptor {
                    physical_width: surface_config.width,
                    physical_height: surface_config.height,
                    scale_factor: window.scale_factor() as f32,
                };

                egui_rpass
                    .add_textures(&device, &queue, &output.textures_delta)
                    .unwrap();
                egui_rpass.remove_textures(output.textures_delta).unwrap();
                egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

                egui_rpass
                    .execute(
                        &mut encoder,
                        &output_view,
                        &paint_jobs,
                        &screen_descriptor,
                        None,
                    )
                    .unwrap();

                queue.submit(iter::once(encoder.finish()));

                output_frame.present();
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(size) => {
                    if size.width > 0 && size.height > 0 {
                        surface_config.width = size.width;
                        surface_config.height = size.height;
                        surface.configure(&device, &surface_config);
                    }
                }
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                event => {
                    state.on_event(&context, &event);
                }
            },
            _ => (),
        }
    });
}
