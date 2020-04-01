use crate::hal::{
    vao_float_builder, BufferId, Font, Shader, VertexArrayEntry, VertexArrayId, BACKEND,
};
use crate::prelude::FlexiTile;
use crate::Result;
use bracket_color::prelude::RGBA;
use bracket_geometry::prelude::PointF;
use glow::HasContext;

pub struct FancyConsoleBackend {
    vertex_buffer: Vec<f32>,
    index_buffer: Vec<i32>,
    vbo: BufferId,
    vao: VertexArrayId,
    ebo: BufferId,
}

impl FancyConsoleBackend {
    pub fn new(_width: usize, _height: usize, gl: &glow::Context) -> FancyConsoleBackend {
        let (vbo, vao, ebo) = FancyConsoleBackend::init_gl_for_console(gl);
        FancyConsoleBackend {
            vertex_buffer: Vec::new(),
            index_buffer: Vec::new(),
            vbo,
            vao,
            ebo,
        }
    }

    fn init_gl_for_console(gl: &glow::Context) -> (BufferId, VertexArrayId, BufferId) {
        vao_float_builder(
            gl,
            &[
                VertexArrayEntry { index: 0, size: 3 }, // Position
                VertexArrayEntry { index: 1, size: 4 }, // Color
                VertexArrayEntry { index: 2, size: 4 }, // Background
                VertexArrayEntry { index: 3, size: 2 }, // Texture Coordinate
                VertexArrayEntry { index: 4, size: 3 }, // Rotation
                VertexArrayEntry { index: 5, size: 2 }, // Scale
            ],
        )
    }

    /// Helper to push a point to the shader.
    #[allow(clippy::too_many_arguments)]
    fn push_point(
        vertex_buffer: &mut Vec<f32>,
        x: f32,
        y: f32,
        fg: RGBA,
        bg: RGBA,
        ux: f32,
        uy: f32,
        rotation: f32,
        screen_x: f32,
        screen_y: f32,
        scale: PointF,
    ) {
        vertex_buffer.extend_from_slice(&[
            x, y, 0.0, fg.r, fg.g, fg.b, fg.a, bg.r, bg.g, bg.b, bg.a, ux, uy, rotation, screen_x,
            screen_y, scale.x, scale.y,
        ]);
    }

    /// Helper to build vertices for the sparse grid.
    #[allow(clippy::too_many_arguments)]
    pub fn rebuild_vertices(
        &mut self,
        height: u32,
        width: u32,
        offset_x: f32,
        offset_y: f32,
        scale: f32,
        scale_center: (i32, i32),
        tiles: &[FlexiTile],
        font_dimensions_glyphs: (u32, u32),
    ) {
        if tiles.is_empty() {
            return;
        }

        self.vertex_buffer.clear();
        self.index_buffer.clear();

        let glyphs_on_font_x = font_dimensions_glyphs.0 as f32;
        let glyphs_on_font_y = font_dimensions_glyphs.1 as f32;
        let glyph_size_x: f32 = 1.0 / glyphs_on_font_x;
        let glyph_size_y: f32 = 1.0 / glyphs_on_font_y;

        let step_x: f32 = scale * 2.0 / width as f32;
        let step_y: f32 = scale * 2.0 / height as f32;

        let mut index_count: i32 = 0;
        let screen_x_start: f32 = -1.0 * scale
            - 2.0 * (scale_center.0 - width as i32 / 2) as f32 * (scale - 1.0) / width as f32;
        let screen_y_start: f32 = -1.0 * scale
            + 2.0 * (scale_center.1 - height as i32 / 2) as f32 * (scale - 1.0) / height as f32;

        for t in tiles.iter() {
            let x = t.position.x;
            let y = t.position.y;

            let screen_x = ((step_x * x) + screen_x_start) + offset_x;
            let screen_y = ((step_y * y) + screen_y_start) + offset_y;
            let fg = t.fg;
            let bg = t.bg;
            let glyph = t.glyph;
            let glyph_x = glyph % font_dimensions_glyphs.0 as u16;
            let glyph_y =
                font_dimensions_glyphs.1 as u16 - (glyph / font_dimensions_glyphs.0 as u16);

            let glyph_left = f32::from(glyph_x) * glyph_size_x;
            let glyph_right = f32::from(glyph_x + 1) * glyph_size_x;
            let glyph_top = f32::from(glyph_y) * glyph_size_y;
            let glyph_bottom = f32::from(glyph_y - 1) * glyph_size_y;

            let rot_center_x = screen_x + (step_x / 2.0);
            let rot_center_y = screen_y + (step_y / 2.0);

            FancyConsoleBackend::push_point(
                &mut self.vertex_buffer,
                screen_x + step_x,
                screen_y + step_y,
                fg,
                bg,
                glyph_right,
                glyph_top,
                t.rotation,
                rot_center_x,
                rot_center_y,
                t.scale,
            );
            FancyConsoleBackend::push_point(
                &mut self.vertex_buffer,
                screen_x + step_x,
                screen_y,
                fg,
                bg,
                glyph_right,
                glyph_bottom,
                t.rotation,
                rot_center_x,
                rot_center_y,
                t.scale,
            );
            FancyConsoleBackend::push_point(
                &mut self.vertex_buffer,
                screen_x,
                screen_y,
                fg,
                bg,
                glyph_left,
                glyph_bottom,
                t.rotation,
                rot_center_x,
                rot_center_y,
                t.scale,
            );
            FancyConsoleBackend::push_point(
                &mut self.vertex_buffer,
                screen_x,
                screen_y + step_y,
                fg,
                bg,
                glyph_left,
                glyph_top,
                t.rotation,
                rot_center_x,
                rot_center_y,
                t.scale,
            );

            self.index_buffer.push(index_count);
            self.index_buffer.push(1 + index_count);
            self.index_buffer.push(3 + index_count);
            self.index_buffer.push(1 + index_count);
            self.index_buffer.push(2 + index_count);
            self.index_buffer.push(3 + index_count);

            index_count += 4;
        }

        let be = BACKEND.lock();
        let gl = be.gl.as_ref().unwrap();
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                &self.vertex_buffer.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                &self.index_buffer.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );
        }
    }

    pub fn gl_draw(&mut self, font: &Font, shader: &Shader, tiles: &[FlexiTile]) -> Result<()> {
        let be = BACKEND.lock();
        let gl = be.gl.as_ref().unwrap();
        unsafe {
            // bind Texture
            font.bind_texture(gl);

            // render container
            shader.useProgram(gl);
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.draw_elements(
                glow::TRIANGLES,
                (tiles.len() * 6) as i32,
                glow::UNSIGNED_INT,
                0,
            );
            gl.disable(glow::BLEND);
        }
        Ok(())
    }
}
