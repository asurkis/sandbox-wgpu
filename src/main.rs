use std::{array, mem};

use sdl3::{
    event::{Event, WindowEvent},
    keyboard::Scancode,
    mouse::MouseButton,
    video::Window,
};
use wgpu::{include_wgsl, wgt};

const STAGING_BUFFER_SIZE: u64 = 1 << 24;

struct App {
    primitive_pipeline_fill_rect: wgpu::RenderPipeline,
    primitive_buffer: wgpu::Buffer,
    staging_buffers: [wgpu::Buffer; 2],
    submission_idx: [Option<wgpu::SubmissionIndex>; 2],
    current_frame: usize,
    quad_idx: wgpu::Buffer,
    surface_config: wgpu::SurfaceConfiguration,
    queue: wgpu::Queue,
    device: wgpu::Device,
    surface: wgpu::Surface<'static>,
    window: Window,
}

#[derive(Clone, Copy, Debug)]
struct PrimitiveRect {
    #[allow(unused)]
    pos: [f32; 4],
    #[allow(unused)]
    color: [f32; 4],
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
            present_mode: wgt::PresentMode::AutoVsync,
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
        let mut quad_idx_map = quad_idx.get_mapped_range_mut(..);
        for (i, &w) in [0u16, 1, 2, 2, 1, 3].iter().enumerate() {
            quad_idx_map[2 * i..][..2].copy_from_slice(&w.to_ne_bytes());
        }
        mem::drop(quad_idx_map);
        quad_idx.unmap();

        let staging_buffers = array::from_fn(|_| {
            device.create_buffer(&wgt::BufferDescriptor {
                label: None,
                size: STAGING_BUFFER_SIZE,
                usage: wgt::BufferUsages::COPY_SRC | wgt::BufferUsages::MAP_WRITE,
                mapped_at_creation: true,
            })
        });

        let primitive_buffer = device.create_buffer(&wgt::BufferDescriptor {
            label: None,
            size: STAGING_BUFFER_SIZE,
            usage: wgt::BufferUsages::COPY_DST | wgt::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let shader_module_desc = include_wgsl!("main.wgsl");
        let shader_module = device.create_shader_module(shader_module_desc);

        let primitive_pipeline_fill_rect =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: None,
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 32,
                        step_mode: wgt::VertexStepMode::Instance,
                        attributes: &[
                            wgt::VertexAttribute {
                                format: wgt::VertexFormat::Float32x4,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgt::VertexAttribute {
                                format: wgt::VertexFormat::Float32x4,
                                offset: 16,
                                shader_location: 1,
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
            primitive_pipeline_fill_rect,
            primitive_buffer,
            staging_buffers,
            submission_idx: array::from_fn(|_| None),
            current_frame: 0,
            quad_idx,
            surface_config,
            queue,
            device,
            surface,
            window,
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

    fn on_frame(&mut self, rects: &[PrimitiveRect]) -> Result<(), wgpu::SurfaceError> {
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
        let render_pass_desc = wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &out_tex_view,
                depth_slice: None,
                resolve_target: None,
                ops: Default::default(),
            })],
            ..Default::default()
        };
        let staging = &self.staging_buffers[self.current_frame];
        let mut mapping = staging.get_mapped_range_mut(..);
        let mut offset = 0;
        for &rect in rects {
            let bytes = self.rect_to_bytes(rect);
            let next_off = offset + bytes.len();
            mapping[offset..next_off].copy_from_slice(&bytes);
            offset = next_off;
        }
        mem::drop(mapping);
        let staging = &self.staging_buffers[self.current_frame];
        staging.unmap();
        encoder.copy_buffer_to_buffer(staging, 0, &self.primitive_buffer, 0, None);

        let mut render_pass = encoder.begin_render_pass(&render_pass_desc);
        render_pass.set_pipeline(&self.primitive_pipeline_fill_rect);
        let primitive_buffer_slice_gpu = self.primitive_buffer.slice(0..offset as u64);
        let quad_idx_slice = self.quad_idx.slice(..);
        render_pass.set_pipeline(&self.primitive_pipeline_fill_rect);
        render_pass.set_index_buffer(quad_idx_slice, wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, primitive_buffer_slice_gpu);
        render_pass.draw_indexed(0..6, 0, 0..rects.len() as u32);
        mem::drop(render_pass);

        let staging = &self.staging_buffers[self.current_frame];
        encoder.map_buffer_on_submit(staging, wgpu::MapMode::Write, .., Result::unwrap);

        let submission_idx = self.queue.submit([encoder.finish()]);
        self.submission_idx[self.current_frame] = Some(submission_idx);
        self.current_frame ^= 1;
        out_tex.present();
        Ok(())
    }

    fn rect_to_bytes(&self, mut rect: PrimitiveRect) -> [u8; 32] {
        let w = self.surface_config.width as f32;
        let h = self.surface_config.height as f32;
        rect.pos[0] = rect.pos[0] / w * 2.0 - 1.0;
        rect.pos[1] = rect.pos[1] / h * -2.0 + 1.0;
        rect.pos[2] = rect.pos[2] / w * 2.0 - 1.0;
        rect.pos[3] = rect.pos[3] / h * -2.0 + 1.0;
        unsafe { mem::transmute(rect) }
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

    const WINDOW_PADDING: f32 = 16.0;
    const GRID_STEP: f32 = 8.0;
    let mut window_pos_og = [120.0, 120.0, 360.0, 360.0];
    let mut window_pos = window_pos_og;
    let mut window_drag = None;
    let mut ctrl_pressed = false;

    'main_loop: loop {
        while let Some(event) = sdl_event_pump.poll_event() {
            match event {
                Event::Quit { .. } => break 'main_loop,
                Event::Window {
                    win_event: WindowEvent::Resized(_, _),
                    ..
                } => {
                    app.on_resize();
                }
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => match scancode {
                    Scancode::Escape => break 'main_loop,
                    Scancode::LCtrl => ctrl_pressed = true,
                    _ => {}
                },
                Event::KeyUp {
                    scancode: Some(Scancode::LCtrl),
                    ..
                } => ctrl_pressed = false,
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x,
                    y,
                    ..
                } => {
                    let mut mask = 0;
                    if window_pos[0] <= x && x <= window_pos[0] + WINDOW_PADDING {
                        mask |= 1;
                    } else if window_pos[2] - WINDOW_PADDING <= x && x <= window_pos[2] {
                        mask |= 4;
                    }
                    if window_pos[1] <= y && y <= window_pos[1] + WINDOW_PADDING {
                        mask |= 2;
                    } else if window_pos[3] - WINDOW_PADDING <= y && y <= window_pos[3] {
                        mask |= 8;
                    }
                    if mask != 0 {
                        window_drag = Some((x, y, mask));
                    }
                }
                Event::MouseButtonUp {
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x,
                    y,
                    ..
                } => {
                    if let Some((x_og, y_og, mask)) = window_drag {
                        let delta = [x - x_og, y - y_og];
                        for i in 0..4 {
                            if 0 != mask & (1 << i) {
                                window_pos[i] = window_pos_og[i] + delta[i & 1];
                                if ctrl_pressed {
                                    window_pos[i] = (window_pos[i] / GRID_STEP).round() * GRID_STEP;
                                }
                            }
                        }
                        window_pos[0] = window_pos[0].max(0.0);
                        window_pos[1] = window_pos[1].max(0.0);
                        window_pos[2] = window_pos[2].min(app.surface_config.width as f32);
                        window_pos[3] = window_pos[3].min(app.surface_config.height as f32);
                        window_pos_og = window_pos;
                        window_drag = None;
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    if let Some((x_og, y_og, mask)) = window_drag {
                        let delta = [x - x_og, y - y_og];
                        for i in 0..4 {
                            if 0 != mask & (1 << i) {
                                window_pos[i] = window_pos_og[i] + delta[i & 1];
                                if ctrl_pressed {
                                    window_pos[i] = (window_pos[i] / GRID_STEP).round() * GRID_STEP;
                                }
                            }
                        }
                        window_pos[0] = window_pos[0].max(0.0);
                        window_pos[1] = window_pos[1].max(0.0);
                        window_pos[2] = window_pos[2].min(app.surface_config.width as f32);
                        window_pos[3] = window_pos[3].min(app.surface_config.height as f32);
                    }
                }
                _ => {}
            }
        }
        let rects = [
            PrimitiveRect {
                pos: window_pos,
                color: [1.0; 4],
            },
            PrimitiveRect {
                pos: [
                    window_pos[0] + WINDOW_PADDING,
                    window_pos[1] + WINDOW_PADDING,
                    window_pos[2] - WINDOW_PADDING,
                    window_pos[3] - WINDOW_PADDING,
                ],
                color: [0.5, 0.5, 0.5, 1.0],
            },
        ];
        match app.on_frame(&rects) {
            Ok(()) => {}
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                app.on_resize();
            }
            Err(err) => panic!("{err}"),
        }
    }
    app.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
}
