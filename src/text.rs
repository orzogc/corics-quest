use crate::async_utils::wait_once;
use crate::resources::*;
use crate::shaders::quad_shader;
use crate::{get_gctx, SCREEN_HEIGHT, SCREEN_WIDTH};

use miniquad::{
    Bindings, BufferSource, BufferType, BufferUsage, GlContext, Pipeline, RenderingBackend,
    UniformsSource,
};

pub struct Text {
    offset: [f32; 2],
    width: i32,
    height: i32,
    shown_chars: i32,
    font: Texture,
    local_buf: Vec<[[f32; 2]; 3]>,
    inst_buf_len: usize,
    bindings: Option<Bindings>,
    quad_pipeline: Pipeline,
}

impl Text {
    pub fn new(res: &Resources, x: i32, y: i32) -> Self {
        Self {
            offset: [x as f32, y as f32],
            width: 0,
            height: 0,
            font: res.font,
            shown_chars: 0,
            local_buf: Vec::new(),
            inst_buf_len: 0,
            bindings: None,
            quad_pipeline: res.quad_pipeline,
        }
    }

    pub fn from_str(gctx: &mut GlContext, res: &Resources, x: i32, y: i32, s: &str) -> Self {
        let mut text = Self::new(res, x, y);
        text.set_text(gctx, res, s);
        text
    }

    pub fn all_chars_shown(&self) -> bool {
        self.shown_chars >= self.local_buf.len() as i32
    }

    pub fn draw(&self, gctx: &mut GlContext) {
        if self.shown_chars <= 0 {
            return;
        }

        if let Some(bindings) = &self.bindings {
            gctx.apply_pipeline(&self.quad_pipeline);
            gctx.apply_bindings(bindings);
            gctx.apply_uniforms(UniformsSource::table(&quad_shader::Uniforms {
                px_src_offset: [0.0, 0.0],
                px_dest_offset: self.offset,
                px_framebuffer_size: [SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32],
                px_texture_size: [self.font.width as f32, self.font.height as f32],
            }));
            gctx.draw(0, 6, self.shown_chars);
        }
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn hide_all_chars(&mut self) {
        self.shown_chars = 0;
    }

    pub async fn reveal(&mut self) {
        self.hide_all_chars();
        while !self.all_chars_shown() {
            wait_once().await;
            self.show_one_char();
        }
    }

    pub fn set_offset(&mut self, x: i32, y: i32) {
        self.offset = [x as f32, y as f32];
    }

    pub fn set_text(&mut self, gctx: &mut GlContext, res: &Resources, s: &str) {
        const FONT_COLUMNS: u32 = 16;
        const FONT_ROWS: u32 = 16;
        let char_width = self.font.width / FONT_COLUMNS;
        let char_height = self.font.height / FONT_ROWS;

        self.local_buf.clear();
        self.width = 0;
        self.height = 0;
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        for c in s.chars() {
            if c == ' ' {
                x += char_width as f32;
            } else if c == '\n' {
                x = 0.0;
                y += char_height as f32;
            } else {
                let c = match c {
                    '!'..='~' => c,
                    '►' => 16 as char,
                    '…' => 255 as char,
                    _ => '?',
                };
                let src_x = (c as u32 % FONT_COLUMNS * char_width) as f32;
                let src_y = (c as u32 / FONT_ROWS * char_height) as f32;
                self.local_buf.push([
                    [char_width as f32, char_height as f32],
                    [src_x, src_y],
                    [x, y],
                ]);
                self.width = self.width.max((x + char_width as f32).trunc() as i32);
                self.height = self.height.max((y + char_height as f32).trunc() as i32);
                x += char_width as f32;
            }
        }

        self.shown_chars = self
            .local_buf
            .len()
            .try_into()
            .expect("local_buf.len() as i32");

        if self.local_buf.is_empty() {
            if let Some(bindings) = self.bindings.take() {
                gctx.delete_buffer(bindings.vertex_buffers[1]);
                self.inst_buf_len = 0;
            }
        } else {
            let needed_len = self.local_buf.len();
            let bindings = self.bindings.get_or_insert_with(|| {
                let inst_buf = gctx.new_buffer(
                    BufferType::VertexBuffer,
                    BufferUsage::Dynamic,
                    BufferSource::empty::<[[f32; 2]; 3]>(needed_len),
                );
                self.inst_buf_len = needed_len;
                Bindings {
                    vertex_buffers: vec![res.quad_vbuf, inst_buf],
                    index_buffer: res.quad_ibuf,
                    images: vec![self.font.tex_id],
                }
            });
            if self.inst_buf_len < needed_len {
                gctx.delete_buffer(bindings.vertex_buffers[1]);
                bindings.vertex_buffers[1] = gctx.new_buffer(
                    BufferType::VertexBuffer,
                    BufferUsage::Dynamic,
                    BufferSource::empty::<[[f32; 2]; 3]>(needed_len),
                );
                self.inst_buf_len = needed_len;
            }
            gctx.buffer_update(
                bindings.vertex_buffers[1],
                BufferSource::slice(&self.local_buf[..]),
            );
        }
    }

    pub fn show_all_chars(&mut self) {
        self.shown_chars = self.local_buf.len() as i32;
    }

    pub fn show_one_char(&mut self) {
        if self.shown_chars < self.local_buf.len() as i32 {
            self.shown_chars += 1;
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        let gctx = get_gctx();

        if let Some(bindings) = &self.bindings {
            gctx.delete_buffer(bindings.vertex_buffers[1]);
        }
    }
}
