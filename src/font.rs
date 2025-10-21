use crate::program;
use std::{
    collections::HashMap,
    io::{BufReader, Cursor},
};
use wgpu::wgt;

const FONT_DATA: &[u8] = include_bytes!("PixelFont1.png");

#[derive(Debug)]
pub struct Font {
    pub texture: wgpu::Texture,
    pub char_size: [u32; 2],
    pub glyphs: HashMap<char, [u32; 2]>,
}

impl Font {
    pub fn new(ctx: &program::Context) -> Self {
        let cursor = Cursor::new(FONT_DATA);
        let reader = BufReader::new(cursor);
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info().unwrap();
        let mut png_data = vec![0; reader.output_buffer_size().unwrap()];
        let png_info = reader.next_frame(&mut png_data).unwrap();

        // Character positions are hard-coded for now
        let mut glyphs = HashMap::new();
        let rows = [
            (112, "0123456789()[]{}<>@#$"),
            (128, "+-*÷%=/\\|~^!?….,'\":;_"),
            (160, "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ"),
            (176, "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"),
            (208, "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            (224, "abcdefghijklmnopqrstuvwxyz"),
        ];
        for (pos_y, row) in rows {
            for (i, c) in row.chars().enumerate() {
                glyphs.insert(c, [16 + 12 * i as u32, pos_y]);
            }
        }

        // Currently we do not optimize texture size
        let texture_size = wgt::Extent3d {
            width: ceil_pow2(png_info.width).max(256),
            height: ceil_pow2(png_info.height),
            depth_or_array_layers: 1,
        };
        let mut texture_data = vec![0; (4 * texture_size.width * texture_size.height) as usize];
        for y in 0..png_info.height as usize {
            for x in 0..png_info.width as usize {
                let off_buf = (x + png_info.height as usize * y) * reader.info().bytes_per_pixel();
                let off_tex = (x + texture_size.width as usize * y) * 4;
                texture_data[off_tex..][..4].fill(if png_data[off_buf] == 21 { 255 } else { 0 });
            }
        }

        let texture = ctx.device.create_texture(&wgt::TextureDescriptor {
            label: Some("Default font texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgt::TextureDimension::D2,
            format: wgt::TextureFormat::Rgba8Unorm,
            usage: wgt::TextureUsages::COPY_DST | wgt::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgt::TextureFormat::Rgba8Unorm],
        });
        let texel_copy_info = wgt::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgt::Origin3d::ZERO,
            aspect: wgt::TextureAspect::All,
        };
        ctx.queue.write_texture(
            texel_copy_info,
            &texture_data,
            wgt::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_size.width),
                rows_per_image: None,
            },
            texture_size,
        );

        Self {
            texture,
            char_size: [12, 16],
            glyphs,
        }
    }
}

fn ceil_pow2(x: u32) -> u32 {
    let clz = (x.max(1) - 1).leading_zeros();
    1 << (32 - clz)
}
