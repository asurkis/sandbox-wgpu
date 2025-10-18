#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PrimitiveVertex {
    pub coord: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Default)]
pub struct PrimitiveList {
    pub idx: Vec<u32>,
    pub vtx: Vec<PrimitiveVertex>,
    pub window_size: [u32; 2],
}

impl PrimitiveList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.vtx.clear();
        self.idx.clear();
    }

    pub fn pos_from_px(&self, pos: [f32; 2]) -> [f32; 2] {
        [
            pos[0] / self.window_size[0] as f32 * 2.0 - 1.0,
            pos[1] / self.window_size[1] as f32 * (-2.0) + 1.0,
        ]
    }

    pub fn fill_rect(&mut self, bounds: [f32; 4], color: [f32; 4]) {
        let pos1 = [bounds[0], bounds[1]];
        let pos2 = [bounds[2], bounds[1]];
        let pos3 = [bounds[0], bounds[3]];
        let pos4 = [bounds[2], bounds[3]];
        let idx1 = self.vtx.len() as u32;
        for coord in [pos1, pos2, pos3, pos4] {
            self.vtx.push(PrimitiveVertex { coord, color });
        }
        self.idx.push(idx1);
        self.idx.push(idx1 + 1);
        self.idx.push(idx1 + 2);
        self.idx.push(idx1 + 3);
        self.idx.push(idx1 + 2);
        self.idx.push(idx1 + 1);
    }

    pub fn fill_rect_px(&mut self, bounds: [f32; 4], color: [f32; 4]) {
        // vertical ordering is flipped
        let [x, y] = self.pos_from_px([bounds[0], bounds[3]]);
        let [z, w] = self.pos_from_px([bounds[2], bounds[1]]);
        self.fill_rect([x, y, z, w], color);
    }
}
