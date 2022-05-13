use crossfont::{GlyphKey, Rasterize};
use tiny_skia::{Color, Pixmap, PixmapPaint, PixmapRef, Transform};

pub struct TitleText {
    title: String,

    font_desc: crossfont::FontDesc,
    font_key: crossfont::FontKey,
    size: crossfont::Size,
    scale: u32,
    metrics: crossfont::Metrics,
    rasterizer: crossfont::Rasterizer,
    color: Color,

    pixmap: Option<Pixmap>,
}

impl std::fmt::Debug for TitleText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TitleText")
            .field("title", &self.title)
            .field("font_desc", &self.font_desc)
            .field("font_key", &self.font_key)
            .field("size", &self.size)
            .field("scale", &self.scale)
            .field("pixmap", &self.pixmap)
            .finish()
    }
}

impl TitleText {
    pub fn new(color: Color) -> Result<Self, crossfont::Error> {
        let title = "".into();
        let scale = 1;

        let font_desc = crossfont::FontDesc::new(
            // "monospace",
            "sans-serif",
            crossfont::Style::Description {
                slant: crossfont::Slant::Normal,
                weight: crossfont::Weight::Normal,
            },
        );

        let mut rasterizer = crossfont::Rasterizer::new(scale as f32, false)?;
        let size = crossfont::Size::new(10.0);
        let font_key = rasterizer.load_font(&font_desc, size)?;

        // Need to load at least one glyph for the face before calling metrics.
        // The glyph requested here ('m' at the time of writing) has no special
        // meaning.
        rasterizer.get_glyph(GlyphKey {
            font_key,
            character: 'm',
            size,
        })?;

        let metrics = rasterizer.metrics(font_key, size)?;

        let mut this = Self {
            title,
            font_desc,
            font_key,
            size,
            scale,
            metrics,
            rasterizer,
            color,
            pixmap: None,
        };

        this.rerender();

        Ok(this)
    }

    fn update_metrics(&mut self) -> Result<(), crossfont::Error> {
        self.rasterizer.get_glyph(GlyphKey {
            font_key: self.font_key,
            character: 'm',
            size: self.size,
        })?;
        self.metrics = self.rasterizer.metrics(self.font_key, self.size)?;
        Ok(())
    }

    pub fn update_scale(&mut self, scale: u32) {
        if self.scale != scale {
            self.rasterizer.update_dpr(scale as f32);
            self.scale = scale;

            self.update_metrics().ok();

            self.rerender();
        }
    }

    pub fn update_title<S: Into<String>>(&mut self, title: S) {
        let title = title.into();
        if self.title != title {
            self.title = title;
            self.rerender();
        }
    }

    pub fn update_color(&mut self, color: Color) {
        if self.color != color {
            self.color = color;
            self.rerender();
        }
    }

    fn rerender(&mut self) {
        let glyphs: Vec<_> = self
            .title
            .chars()
            .filter_map(|character| {
                let key = GlyphKey {
                    character,
                    font_key: self.font_key,
                    size: self.size,
                };

                self.rasterizer
                    .get_glyph(key)
                    .map(|glyph| (key, glyph))
                    .ok()
            })
            .collect();

        if glyphs.is_empty() {
            self.pixmap = None;
            return;
        }

        let width = glyphs
            .iter()
            .fold(0, |w, (_, g)| w + (g.left + g.width).max(5));
        let height = self.metrics.line_height.round() as i32;

        let mut pixmap = if let Some(p) = Pixmap::new(width as u32, height as u32) {
            p
        } else {
            self.pixmap = None;
            return;
        };
        // pixmap.fill(Color::from_rgba8(255, 0, 0, 55));

        let mut caret = 0;
        let mut last_glyph = None;

        for (key, glyph) in glyphs {
            let mut buffer = Vec::with_capacity(glyph.width as usize * 4);

            let glyph_buffer = match &glyph.buffer {
                crossfont::BitmapBuffer::Rgb(v) => v.chunks(3),
                crossfont::BitmapBuffer::Rgba(v) => v.chunks(4),
            };

            for px in glyph_buffer {
                let alpha = if let Some(alpha) = px.get(3) {
                    *alpha as f32 / 255.0
                } else {
                    let r = px[0] as f32 / 255.0;
                    let g = px[1] as f32 / 255.0;
                    let b = px[2] as f32 / 255.0;
                    (r + g + b) / 3.0
                };

                let mut color = self.color;
                color.set_alpha(alpha);
                let color = color.premultiply().to_color_u8();

                buffer.push(color.red());
                buffer.push(color.red());
                buffer.push(color.green());
                buffer.push(color.alpha());
            }

            if let Some(last) = last_glyph {
                let (x, _) = self.rasterizer.kerning(last, key);
                caret += x as i32;
            }

            if let Some(pixmap_glyph) =
                PixmapRef::from_bytes(&buffer, glyph.width as _, glyph.height as _)
            {
                pixmap.draw_pixmap(
                    glyph.left + caret,
                    height - glyph.top + self.metrics.descent.round() as i32,
                    pixmap_glyph,
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            }

            caret += glyph.advance.0;

            last_glyph = Some(key);
        }

        self.pixmap = Some(pixmap);
    }

    pub fn pixmap(&self) -> Option<&Pixmap> {
        self.pixmap.as_ref()
    }
}
