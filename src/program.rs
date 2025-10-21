use sdl3::{EventPump, video::Window};
use std::{array, mem};
use wgpu::wgt;

use crate::primitives::{PrimitiveList, Vertex};

pub const STAGING_BUFFER_SIZE: u64 = 1 << 24;

pub struct Context {
    pub primitive_pipeline: wgpu::RenderPipeline,
    pub primitive_pipeline_layout: wgpu::PipelineLayout,
    pub primitive_pipeline_bind_group_layout: wgpu::BindGroupLayout,
    pub primitive_buffer: wgpu::Buffer,
    pub staging_buffers: [wgpu::Buffer; 2],
    pub submission_idx: [Option<wgpu::SubmissionIndex>; 2],
    pub white_tex_view: wgpu::TextureView,
    pub white_tex: wgpu::Texture,
    pub default_sampler: wgpu::Sampler,
    pub current_frame: usize,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub surface: wgpu::Surface<'static>,
    pub window: Window,
    pub event_pump: EventPump,
}

impl Context {
    pub async fn new() -> Self {
        let sdl = sdl3::init().unwrap();
        let sdl_video = sdl.video().unwrap();
        let event_pump = sdl.event_pump().unwrap();
        let window = sdl_video
            .window("Main window", 1280, 720)
            .resizable()
            .build()
            .unwrap();
        let wgpu_instance = wgpu::Instance::new(&wgt::InstanceDescriptor {
            #[cfg(target_os = "windows")]
            backends: wgt::Backends::DX12,
            #[cfg(not(target_os = "windows"))]
            backends: wgt::Backends::PRIMARY,
            flags: wgt::InstanceFlags::DEBUG
                | wgt::InstanceFlags::VALIDATION
                | wgt::InstanceFlags::GPU_BASED_VALIDATION
                | wgt::InstanceFlags::VALIDATION_INDIRECT_CALL,
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
        });
        let window_size = window.size_in_pixels();
        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap();
            wgpu_instance.create_surface_unsafe(target).unwrap()
        };
        let adapter = wgpu_instance
            .request_adapter(&wgt::RequestAdapterOptions {
                power_preference: wgt::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|it| it.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgt::SurfaceConfiguration {
            usage: wgt::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.0.max(1),
            height: window_size.1.max(1),
            present_mode: wgt::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surface_config);

        let default_sampler = device.create_sampler(&Default::default());

        let white_tex_size = wgt::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let white_tex = device.create_texture(&wgt::TextureDescriptor {
            label: Some("White texture"),
            size: white_tex_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgt::TextureDimension::D2,
            format: wgt::TextureFormat::Rgba8Unorm,
            usage: wgt::TextureUsages::COPY_DST | wgt::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgt::TextureFormat::Rgba8Unorm],
        });
        let white_tex_view = white_tex.create_view(&Default::default());
        queue.write_texture(
            wgt::TexelCopyTextureInfo {
                texture: &white_tex,
                mip_level: 0,
                origin: wgt::Origin3d::ZERO,
                aspect: wgt::TextureAspect::All,
            },
            &[255; 4],
            Default::default(),
            white_tex_size,
        );

        let staging_buffers = array::from_fn(|i| {
            device.create_buffer(&wgt::BufferDescriptor {
                label: Some(&format!("Staging buffer {i}")),
                size: STAGING_BUFFER_SIZE,
                usage: wgt::BufferUsages::COPY_SRC | wgt::BufferUsages::MAP_WRITE,
                mapped_at_creation: true,
            })
        });

        let primitive_buffer = device.create_buffer(&wgt::BufferDescriptor {
            label: Some("Primitive buffer"),
            size: STAGING_BUFFER_SIZE,
            usage: wgt::BufferUsages::COPY_DST
                | wgt::BufferUsages::VERTEX
                | wgt::BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        let shader_module_desc = wgpu::include_wgsl!("primitives.wgsl");
        let shader_module = device.create_shader_module(shader_module_desc);

        let primitive_pipeline_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primitive pipeline bind group layout"),
                entries: &[
                    wgt::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgt::ShaderStages::FRAGMENT,
                        ty: wgt::BindingType::Texture {
                            sample_type: wgt::TextureSampleType::Float { filterable: false },
                            view_dimension: wgt::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgt::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgt::ShaderStages::FRAGMENT,
                        ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let primitive_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Primitive pipeline layout"),
                bind_group_layouts: &[&primitive_pipeline_bind_group_layout],
                push_constant_ranges: &[],
            });

        let primitive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primitive pipeline"),
            layout: Some(&primitive_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<Vertex>() as u64,
                    step_mode: wgt::VertexStepMode::Vertex,
                    attributes: &[
                        wgt::VertexAttribute {
                            format: wgt::VertexFormat::Float32x2,
                            offset: mem::offset_of!(Vertex, coord) as u64,
                            shader_location: 0,
                        },
                        wgt::VertexAttribute {
                            format: wgt::VertexFormat::Float32x4,
                            offset: mem::offset_of!(Vertex, color) as u64,
                            shader_location: 1,
                        },
                        wgt::VertexAttribute {
                            format: wgt::VertexFormat::Float32x2,
                            offset: mem::offset_of!(Vertex, tex_coord) as u64,
                            shader_location: 2,
                        },
                    ],
                }],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgt::BlendState::ALPHA_BLENDING),
                    write_mask: wgt::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            primitive_pipeline,
            primitive_pipeline_layout,
            primitive_pipeline_bind_group_layout,
            primitive_buffer,
            staging_buffers,
            submission_idx: array::from_fn(|_| None),
            white_tex_view,
            white_tex,
            default_sampler,
            current_frame: 0,
            surface_config,
            queue,
            device,
            surface,
            window,
            event_pump,
        }
    }

    pub fn on_resize(&mut self) {
        let (width, height) = self.window.size_in_pixels();
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn on_frame(&mut self, primitives: &PrimitiveList) -> Result<(), wgpu::SurfaceError> {
        let out_tex = self.surface.get_current_texture()?;
        let out_tex_view = out_tex.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        if let Some(idx) = mem::take(&mut self.submission_idx[self.current_frame]) {
            let poll_type = wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            };
            self.device.poll(poll_type).unwrap();
        }
        let staging = &self.staging_buffers[self.current_frame];
        let mut mapping = staging.get_mapped_range_mut(..);
        let off_vtx = 0;
        let (off_idx, count_vtx) = calc_count(off_vtx, &primitives.vtx);
        let (off_end, count_idx) = calc_count(off_idx, &primitives.idx);
        for (i, v) in primitives.vtx[..count_vtx].iter().enumerate() {
            let mut off = off_vtx + i * mem::size_of_val(v);
            for x in v.coord {
                let size = mem::size_of_val(&x);
                mapping[off..][..size].copy_from_slice(&x.to_ne_bytes());
                off += size;
            }
            for x in v.tex_coord {
                let size = mem::size_of_val(&x);
                mapping[off..][..size].copy_from_slice(&x.to_ne_bytes());
                off += size;
            }
            for x in v.color {
                let size = mem::size_of_val(&x);
                mapping[off..][..size].copy_from_slice(&x.to_ne_bytes());
                off += size;
            }
        }
        for (i, v) in primitives.idx[..count_idx].iter().enumerate() {
            let off = off_idx + i * mem::size_of_val(v);
            mapping[off..][..4].copy_from_slice(&v.to_ne_bytes());
        }
        let off_vtx = off_vtx as u64;
        let off_idx = off_idx as u64;
        let off_end = off_end as u64;
        mem::drop(mapping);
        let staging = &self.staging_buffers[self.current_frame];
        encoder.copy_buffer_to_buffer(staging, 0, &self.primitive_buffer, 0, Some(off_end));
        staging.unmap();

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Primitive render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &out_tex_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgt::Operations {
                    load: wgt::LoadOp::Clear(wgt::Color::BLUE),
                    store: wgt::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        let buf_slice_vtx = self.primitive_buffer.slice(off_vtx..off_idx);
        let buf_slice_idx = self.primitive_buffer.slice(off_idx..off_end);
        render_pass.set_pipeline(&self.primitive_pipeline);
        render_pass.set_index_buffer(buf_slice_idx, wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, buf_slice_vtx);
        for cmd in &primitives.commands {
            let texture_view = match cmd.texture {
                None => &self.white_tex_view,
                Some(ref tex) => &tex.create_view(&Default::default()),
            };
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.primitive_pipeline_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                    },
                ],
            });
            render_pass.set_bind_group(0, &bind_group, &[]);
            let cmd_idx_off = count_idx.min(cmd.idx_off) as u32;
            let cmd_idx_cnt = count_idx.min(cmd.idx_off + cmd.idx_cnt) as u32;
            render_pass.draw_indexed(cmd_idx_off..cmd_idx_cnt, 0, 0..1);
        }
        mem::drop(render_pass);

        let staging = &self.staging_buffers[self.current_frame];
        encoder.map_buffer_on_submit(staging, wgpu::MapMode::Write, .., Result::unwrap);

        let submission_idx = self.queue.submit([encoder.finish()]);
        self.submission_idx[self.current_frame] = Some(submission_idx);
        self.current_frame ^= 1;
        out_tex.present();
        Ok(())
    }
}

fn calc_count<T>(curr_off: usize, arr: &[T]) -> (usize, usize) {
    let size = mem::size_of::<T>();
    let remaining = STAGING_BUFFER_SIZE as usize - curr_off;
    let count = arr.len().min(remaining / size);
    let next_off = curr_off + count * size;
    (next_off, count)
}
