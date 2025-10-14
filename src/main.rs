use sdl3::{event::Event, keyboard::Scancode, video::Window};
use wgpu::{include_wgsl, wgt};

struct App {
    window: Window,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    quad_idx: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl App {
    async fn new(window: Window) -> Self {
        let wgpu = wgpu::Instance::new(&wgt::InstanceDescriptor {
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
            wgpu.create_surface_unsafe(target).unwrap()
        };
        let adapter = wgpu
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
            present_mode: wgt::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surface_config);

        let quad_idx = device.create_buffer(&wgt::BufferDescriptor {
            label: None,
            size: 12,
            usage: wgt::BufferUsages::INDEX,
            mapped_at_creation: true,
        });
        {
            let mut quad_idx_map = quad_idx.get_mapped_range_mut(..);
            for (i, &w) in [0u16, 1, 2, 2, 1, 3].iter().enumerate() {
                quad_idx_map[2 * i..][..2].copy_from_slice(&w.to_ne_bytes());
            }
        }
        quad_idx.unmap();

        let shader_module_desc = include_wgsl!("main.wgsl");
        let shader_module = device.create_shader_module(shader_module_desc);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgt::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgt::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgt::PolygonMode::Fill,
                conservative: false,
            },
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
            window,

            surface,
            device,
            queue,
            surface_config,

            quad_idx,
            pipeline,
        }
    }

    fn on_resize(&mut self) {
        let (width, height) = self.window.size_in_pixels();
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    fn on_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        let out_tex = self.surface.get_current_texture()?;
        let out_tex_view = out_tex.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_tex_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgt::Operations {
                        load: wgt::LoadOp::Clear(wgt::Color::BLUE),
                        store: wgt::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            let quad_idx_slice = self.quad_idx.slice(..);
            render_pass.set_index_buffer(quad_idx_slice, wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        out_tex.present();
        Ok(())
    }
}

fn main() {
    let sdl = sdl3::init().unwrap();
    let sdl_video = sdl.video().unwrap();
    let mut sdl_event_pump = sdl.event_pump().unwrap();
    let sdl_window = sdl_video
        .window("Main window", 1280, 720)
        .resizable()
        .build()
        .unwrap();
    let mut app = pollster::block_on(App::new(sdl_window.clone()));
    'main_loop: loop {
        while let Some(event) = sdl_event_pump.poll_event() {
            match event {
                Event::Quit { .. } => break 'main_loop,
                Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => break 'main_loop,
                _ => {}
            }
        }
        match app.on_frame() {
            Ok(()) => {}
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                app.on_resize();
            }
            Err(err) => panic!("{err}"),
        }
    }
}
