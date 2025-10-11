use std::{sync::Arc, time::Instant};

use wgpu::wgt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

#[derive(Default)]
struct AppTop {
    window: Option<Arc<Window>>,
    wgpu: Option<AppWgpu>,
}

struct AppWgpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
}

impl AppWgpu {
    async fn new(window: Arc<Window>) -> Self {
        let ts_init_begin = Instant::now();
        let wgpu = wgpu::Instance::new(&wgt::InstanceDescriptor {
            // #[cfg(target_os = "windows")]
            // backends: wgt::Backends::DX12,
            // #[cfg(not(target_os = "windows"))]
            backends: wgt::Backends::VULKAN,
            // flags: wgt::InstanceFlags::DEBUG
            //     | wgt::InstanceFlags::VALIDATION
            //     | wgt::InstanceFlags::GPU_BASED_VALIDATION
            //     | wgt::InstanceFlags::VALIDATION_INDIRECT_CALL,
            flags: wgt::InstanceFlags::empty(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
        });
        let ts_init_instance = Instant::now();
        let window_size = window.inner_size();
        let surface = wgpu.create_surface(window).unwrap();
        let ts_init_surface = Instant::now();
        let adapter = wgpu
            .request_adapter(&wgt::RequestAdapterOptions {
                power_preference: wgt::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let ts_init_adapter = Instant::now();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let ts_init_device = Instant::now();

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
            width: window_size.width.max(1),
            height: window_size.height.max(1),
            present_mode: wgt::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surface_config);
        let ts_init_end = Instant::now();
        print!(
            "Wgpu init took {} seconds. Of them:\n",
            (ts_init_end - ts_init_begin).as_secs_f64()
        );
        print!(
            "    instance init took {} seconds\n",
            (ts_init_instance - ts_init_begin).as_secs_f64()
        );
        print!(
            "    surface  init took {} seconds\n",
            (ts_init_surface - ts_init_instance).as_secs_f64()
        );
        print!(
            "    adapter  init took {} seconds\n",
            (ts_init_adapter - ts_init_surface).as_secs_f64()
        );
        print!(
            "    device   init took {} seconds\n",
            (ts_init_device - ts_init_adapter).as_secs_f64()
        );
        print!(
            "    config        took {} seconds\n",
            (ts_init_end - ts_init_device).as_secs_f64()
        );

        Self {
            surface,
            device,
            queue,
            surface_config,
        }
    }
}

macro_rules! field {
    ($this:ident.$name:ident !! $ret_stmt:stmt) => {
        let Some(ref mut $name) = $this.$name else {
            $ret_stmt
        };
    };
}

impl AppTop {
    fn on_resize(&mut self) {
        field!(self.window !! return);
        field!(self.wgpu !! return);

        let size = window.inner_size();
        if size.width > 0 && size.height > 0 {
            wgpu.surface_config.width = size.width;
            wgpu.surface_config.height = size.height;
            wgpu.surface.configure(&wgpu.device, &wgpu.surface_config);
        }
    }

    fn on_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        field!(self.window !! return Ok(()));
        field!(self.wgpu !! return Ok(()));

        window.request_redraw();
        let out_tex = wgpu.surface.get_current_texture()?;
        let out_tex_view = out_tex.texture.create_view(&Default::default());
        let mut encoder = wgpu.device.create_command_encoder(&Default::default());
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        }
        wgpu.queue.submit(std::iter::once(encoder.finish()));
        out_tex.present();
        Ok(())
    }
}

impl ApplicationHandler for AppTop {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );
        let self_wgpu = pollster::block_on(AppWgpu::new(window.clone()));
        self.window = Some(window);
        self.wgpu = Some(self_wgpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::RedrawRequested => match self.on_frame() {
                Ok(()) => {}
                Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => self.on_resize(),
                Err(err) => panic!("{err}"),
            },
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = AppTop::default();
    event_loop.run_app(&mut app).unwrap();
}
