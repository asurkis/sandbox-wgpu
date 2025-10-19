#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PrimitiveVertex {
    pub coord: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Default)]
pub struct PrimitiveList {
    pub immediate_indices: bool,
    pub px_space: bool,
    pub color: [f32; 4],
    pub window_size: [u32; 2],

    pub idx: Vec<u32>,
    pub vtx: Vec<PrimitiveVertex>,
}

impl PrimitiveList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.immediate_indices = false;
        self.px_space = false;
        self.color = [1.0; 4];
        self.idx.clear();
        self.vtx.clear();
    }

    pub fn pos_from_px(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        let x = x / self.window_size[0] as f32 * 2.0 - 1.0;
        let y = y / self.window_size[1] as f32 * (-2.0) + 1.0;
        [x, y]
    }

    // OpenGL 1 like API
    #[allow(unused)]
    pub fn vertex_2f(&mut self, mut coord: [f32; 2]) -> u32 {
        if self.px_space {
            coord = self.pos_from_px(coord);
        }
        let idx = self.vertex_inner(coord);
        if self.immediate_indices {
            self.idx.push(idx);
        }
        idx
    }

    fn vertex_inner(&mut self, coord: [f32; 2]) -> u32 {
        let idx = self.vtx.len() as u32;
        let color = self.color;
        self.vtx.push(PrimitiveVertex { coord, color });
        idx
    }

    pub fn rect_4f(&mut self, [x1, y1, x2, y2]: [f32; 4]) {
        if self.px_space {
            // vertical ordering is flipped
            let [x3, y3] = self.pos_from_px([x1, y2]);
            let [x4, y4] = self.pos_from_px([x2, y1]);
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
        self.idx.push(idx1);
        self.idx.push(idx2);
        self.idx.push(idx3);
        self.idx.push(idx4);
        self.idx.push(idx3);
        self.idx.push(idx2);
    }
}
