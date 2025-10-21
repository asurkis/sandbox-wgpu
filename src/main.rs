mod font;
mod primitives;
mod program;

use crate::primitives::PrimitiveList;
use sdl3::{
    event::{Event, WindowEvent},
    keyboard::Scancode,
    mouse::MouseButton,
};

fn main() {
    let mut program_ctx = pollster::block_on(program::Context::new());
    let mut primitives = PrimitiveList::default();
    let font = font::Font::new(&program_ctx);

    const WINDOW_PADDING: f32 = 8.0;
    const GRID_STEP: f32 = 8.0;
    let mut window_pos = [120.0, 120.0, 360.0, 360.0];
    let mut window_pos_og = window_pos;
    let mut window_drag = None;
    let mut ctrl_pressed = false;

    'main_loop: loop {
        while let Some(event) = program_ctx.event_pump.poll_event() {
            match event {
                Event::Quit { .. } => break 'main_loop,
                Event::Window {
                    win_event: WindowEvent::Resized(_, _),
                    ..
                } => {
                    program_ctx.on_resize();
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
                        window_pos[2] = window_pos[2].min(program_ctx.surface_config.width as f32);
                        window_pos[3] = window_pos[3].min(program_ctx.surface_config.height as f32);
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
                        window_pos[2] = window_pos[2].min(program_ctx.surface_config.width as f32);
                        window_pos[3] = window_pos[3].min(program_ctx.surface_config.height as f32);
                    }
                }
                _ => {}
            }
        }

        primitives.clear();
        primitives.window_size = [
            program_ctx.surface_config.width,
            program_ctx.surface_config.height,
        ];

        primitives.texture = Some(font.texture.clone());
        primitives.px_space = false;
        primitives.color = [1.0, 0.0, 0.0, 1.0];
        primitives.tex_coord = [0.0, 1.0];
        let idx1 = primitives.vertex_2f([0.0, 0.0]);
        primitives.tex_coord = [1.0, 1.0];
        let idx2 = primitives.vertex_2f([1.0, 0.0]);
        primitives.tex_coord = [0.0, 0.0];
        let idx3 = primitives.vertex_2f([0.0, 1.0]);
        primitives.tex_coord = [1.0, 0.0];
        let idx4 = primitives.vertex_2f([1.0, 1.0]);
        primitives.push_index(idx1);
        primitives.push_index(idx2);
        primitives.push_index(idx3);
        primitives.push_index(idx4);
        primitives.push_index(idx3);
        primitives.push_index(idx2);

        primitives.texture = None;
        primitives.px_space = true;
        primitives.color = [1.0; 4];
        primitives.rect_4f(window_pos);
        primitives.color = [0.5, 0.5, 0.5, 1.0];
        primitives.rect_4f([
            window_pos[0] + WINDOW_PADDING,
            window_pos[1] + WINDOW_PADDING,
            window_pos[2] - WINDOW_PADDING,
            window_pos[3] - WINDOW_PADDING,
        ]);
        match program_ctx.on_frame(&primitives) {
            Ok(()) => {}
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                program_ctx.on_resize();
            }
            Err(err) => panic!("{err}"),
        }
    }
    program_ctx
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
}
