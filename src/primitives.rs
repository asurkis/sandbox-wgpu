use crate::font::Font;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vertex {
    pub coord: [f32; 2],
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Default)]
pub struct Command {
    pub texture: Option<wgpu::Texture>,
    pub idx_off: usize,
    pub idx_cnt: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PrimitiveList {
    pub texture: Option<wgpu::Texture>,

    pub immediate_indices: bool,
    pub px_space: bool,
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
    pub window_size: [u32; 2],

    pub commands: Vec<Command>,
    pub idx: Vec<u32>,
    pub vtx: Vec<Vertex>,
}

impl PrimitiveList {
    pub fn clear(&mut self) {
        self.texture = None;
        self.immediate_indices = false;
        self.px_space = false;
        self.color = [0.0; 4];
        self.commands.clear();
        self.idx.clear();
        self.vtx.clear();
    }

    pub fn px_to_pos(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        let x = x / self.window_size[0] as f32 * 2.0 - 1.0;
        let y = y / self.window_size[1] as f32 * (-2.0) + 1.0;
        [x, y]
    }

    fn last_command(&mut self) -> &mut Command {
        let need_push = match self.commands.last() {
            None => true,
            Some(cmd) => cmd.idx_cnt != 0 && cmd.texture != self.texture,
        };
        if need_push {
            self.commands.push(Command {
                texture: self.texture.clone(),
                idx_off: self.idx.len(),
                idx_cnt: 0,
            });
        }
        self.commands.last_mut().unwrap()
    }

    pub fn push_index(&mut self, idx: u32) {
        self.last_command().idx_cnt += 1;
        self.idx.push(idx);
    }

    // OpenGL 1 like API
    #[allow(unused)]
    pub fn vertex_2f(&mut self, mut coord: [f32; 2]) -> u32 {
        if self.px_space {
            coord = self.px_to_pos(coord);
        }
        let idx = self.vertex_inner(coord);
        if self.immediate_indices {
            self.push_index(idx);
        }
        idx
    }

    fn vertex_inner(&mut self, coord: [f32; 2]) -> u32 {
        let idx = self.vtx.len() as u32;
        self.vtx.push(Vertex {
            coord,
            tex_coord: self.tex_coord,
            color: self.color,
        });
        idx
    }

    pub fn rect_f(&mut self, [x1, y1, x2, y2]: [f32; 4]) {
        if self.px_space {
            // vertical ordering is flipped
            let [x3, y3] = self.px_to_pos([x1, y2]);
            let [x4, y4] = self.px_to_pos([x2, y1]);
            self.rect_inner([x3, y3, x4, y4]);
        } else {
            self.rect_inner([x1, y1, x2, y2]);
        }
    }

    fn rect_inner(&mut self, [x1, y1, x2, y2]: [f32; 4]) {
        let idx1 = self.vertex_inner([x1, y1]);
        let idx2 = self.vertex_inner([x2, y1]);
        let idx3 = self.vertex_inner([x1, y2]);
        let idx4 = self.vertex_inner([x2, y2]);
        self.last_command().idx_cnt += 6;
        self.idx.push(idx1);
        self.idx.push(idx2);
        self.idx.push(idx3);
        self.idx.push(idx4);
        self.idx.push(idx3);
        self.idx.push(idx2);
    }

    pub fn image_rect_i(
        &mut self,
        [dst_x, dst_y]: [i32; 2],
        [src_x, src_y]: [i32; 2],
        [size_x, size_y]: [u32; 2],
    ) {
        let Some(tex) = self.texture.as_ref() else {
            return;
        };
        let tw = tex.size().width;
        let th = tex.size().height;
        let [dst1_x, dst1_y] = [dst_x, dst_y];
        let [dst2_x, dst2_y] = [dst_x + size_x as i32, dst_y + size_y as i32];
        let [src1_x, src1_y] = [src_x, src_y];
        let [src2_x, src2_y] = [src_x + size_x as i32, src_y + size_y as i32];
        let [x1, y2] = self.px_to_pos([dst1_x as f32, dst1_y as f32]);
        let [x2, y1] = self.px_to_pos([dst2_x as f32, dst2_y as f32]);
        let [u1, v2] = [src1_x as f32 / tw as f32, src1_y as f32 / th as f32];
        let [u2, v1] = [src2_x as f32 / tw as f32, src2_y as f32 / th as f32];
        let verts = [
            [x1, y1, u1, v1],
            [x2, y1, u2, v1],
            [x1, y2, u1, v2],
            [x2, y2, u2, v2],
        ];
        let idx1 = self.vtx.len() as u32;
        self.last_command().idx_cnt += 6;
        for [x, y, u, v] in verts {
            self.vtx.push(Vertex {
                coord: [x, y],
                tex_coord: [u, v],
                color: self.color,
            });
        }
        self.idx.push(idx1);
        self.idx.push(idx1 + 1);
        self.idx.push(idx1 + 2);
        self.idx.push(idx1 + 3);
        self.idx.push(idx1 + 2);
        self.idx.push(idx1 + 1);
    }

    pub fn text_i(&mut self, font: &Font, [start_x, start_y]: [i32; 2], text: &str) {
        let old_texture = std::mem::take(&mut self.texture);
        self.texture = Some(font.texture.clone());

        let mut off_x = 0;
        let mut off_y = 0;
        for c in text.chars() {
            match c {
                ' ' => off_x += font.glyph_size[0] as i32,
                '\n' => {
                    off_x = 0;
                    off_y += font.glyph_size[1] as i32;
                }
                c => {
                    self.last_command().idx_cnt += 6;
                    let dst_x = start_x + off_x;
                    let dst_y = start_y + off_y;
                    let glyph = font.glyphs.get(&c).unwrap_or(&font.fallback_glyph);
                    let src_x = glyph[0] as i32;
                    let src_y = glyph[1] as i32;
                    self.image_rect_i([dst_x, dst_y], [src_x, src_y], font.glyph_size);
                    off_x += font.glyph_size[0] as i32;
                }
            }
        }

        self.texture = old_texture;
    }
}
