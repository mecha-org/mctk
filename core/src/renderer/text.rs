use std::collections::HashMap;

use cosmic_text::fontdb::Database;
use cosmic_text::{
    Align as CosmicAlign, Attrs, AttrsList, Buffer, CacheKey, Color as FontColor, Family,
    FontSystem, LayoutGlyph, Metrics, Shaping, Stretch, Style, SubpixelBin, Weight, Wrap,
};
use femtovg::renderer::OpenGl;
use femtovg::{
    Align, Atlas, Canvas, DrawCommand, ErrorKind, GlyphDrawCommands, ImageFlags, ImageId,
    ImageSource, Paint, Quad, Renderer,
};
use imgref::{Img, ImgRef};
use rgb::RGBA8;
use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::{Format, Vector};

use crate::font_cache::{
    DEFAULT_FONT_SIZE, DEFAULT_LINE_HEIGHT, GLYPH_MARGIN, GLYPH_PADDING, TEXTURE_SIZE,
};
use crate::renderables::text::Instance;
use crate::{Pos, Scale};

// const DEFAULT_FONT_SIZE: f32= 12.;
// const DEFAULT_LINE_HEIGHT: f32 = 16.;
// const GLYPH_PADDING: u32 = 0;
// const GLYPH_MARGIN: u32 = 0;
// const TEXTURE_SIZE: usize = 512;

#[derive(Default, Debug, Clone, Copy)]
pub struct TextConfig {
    pub hint: bool,
    pub subpixel: bool,
}

pub struct FontTexture {
    atlas: Atlas,
    image_id: ImageId,
}

#[derive(Copy, Clone, Debug)]
pub struct RenderedGlyph {
    texture_index: usize,
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
    atlas_x: u32,
    atlas_y: u32,
    color_glyph: bool,
}

pub struct TextRenderer {
    pub font_system: FontSystem,
    pub buffer: Buffer,
    scale_context: ScaleContext,
    rendered_glyphs: HashMap<CacheKey, Option<RenderedGlyph>>,
    glyph_textures: Vec<FontTexture>,
}

impl TextRenderer {
    pub fn new(fonts: Database) -> Self {
        let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_owned());
        let mut font_system = FontSystem::new_with_locale_and_db(locale, fonts);
        let fs = &mut font_system;
        let buffer = Buffer::new(fs, Metrics::new(DEFAULT_FONT_SIZE, DEFAULT_LINE_HEIGHT));

        Self {
            font_system,
            buffer,
            scale_context: ScaleContext::default(),
            rendered_glyphs: HashMap::new(),
            glyph_textures: vec![],
        }
    }

    pub fn clear(&mut self) {
        self.rendered_glyphs.clear();
        self.glyph_textures.clear();
    }

    pub fn draw_text(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        instance: Instance,
    ) -> Result<Vec<(FontColor, GlyphDrawCommands)>, ErrorKind> {
        let Instance {
            pos,
            scale,
            color,
            align,
            font,
            weight,
            font_size,
            line_height,
            text,
        } = instance;

        let fs = &mut self.font_system;
        let buffer = &mut self.buffer;

        buffer.set_metrics(fs, Metrics::new(font_size, line_height));

        let mut attrs = Attrs::new()
            .weight(Weight(weight as u16))
            .stretch(Stretch::Normal)
            .style(Style::Normal)
            .color(FontColor::rgba(
                color.r as u8,
                color.g as u8,
                color.b as u8,
                (color.a * 255.) as u8,
            ));

        if font.is_some() {
            attrs = attrs.family(Family::Name(font.as_ref().unwrap()));
        }

        buffer.set_wrap(fs, Wrap::None);
        buffer.set_text(fs, &text, attrs, Shaping::Advanced);
        buffer.set_size(fs, scale.width, scale.height);

        for line in buffer.lines.iter_mut() {
            // TODO spans
            line.set_attrs_list(AttrsList::new(attrs));
            line.set_align(match align {
                Align::Left => Some(CosmicAlign::Left),
                Align::Center => Some(CosmicAlign::Center),
                Align::Right => Some(CosmicAlign::Right),
            });
        }

        buffer.shape_until(fs, i32::MAX);

        let config = TextConfig {
            hint: true,
            subpixel: true,
        };

        self.fill_to_cmds(canvas, scale, pos, (0., 0.), config)
    }

    pub fn measure_text(
        &mut self,
        instance: Instance,
    ) -> (Option<f32>, Option<f32>, Vec<LayoutGlyph>) {
        let Instance {
            pos,
            scale,
            align,
            font,
            weight,
            font_size,
            line_height,
            text,
            ..
        } = instance;

        let fs = &mut self.font_system;
        let buffer = &mut self.buffer;

        buffer.set_metrics(fs, Metrics::new(font_size, line_height));

        let mut attrs = Attrs::new()
            .weight(Weight(weight as u16))
            .stretch(Stretch::Normal)
            .style(Style::Normal);

        if font.is_some() {
            attrs = attrs.family(Family::Name(font.as_ref().unwrap()));
        }

        buffer.set_wrap(fs, Wrap::None);
        buffer.set_text(fs, &text, attrs, Shaping::Advanced);
        buffer.set_size(fs, scale.width, scale.height);

        for line in buffer.lines.iter_mut() {
            // TODO spans
            line.set_attrs_list(AttrsList::new(attrs));
            line.set_align(match align {
                Align::Left => Some(CosmicAlign::Left),
                Align::Center => Some(CosmicAlign::Center),
                Align::Right => Some(CosmicAlign::Right),
            });
        }

        buffer.shape_until(fs, i32::MAX);

        let config = TextConfig {
            hint: true,
            subpixel: true,
        };

        let (w, h, glyphs) = self.measure_glyphs(scale, pos, (0., 0.), config);
        (Some(w), Some(h), glyphs)
    }

    pub fn measure_glyphs(
        &mut self,
        scale: Scale,
        position: Pos,
        justify: (f32, f32),
        config: TextConfig,
    ) -> (f32, f32, Vec<LayoutGlyph>) {
        let fs = &mut self.font_system;
        let buffer = &mut self.buffer;
        let rendered_glyphs = &mut self.rendered_glyphs;

        let lines = buffer.layout_runs().filter(|run| run.line_w != 0.0).count();
        let total_height = lines as f32 * buffer.metrics().line_height;
        let mut total_width: f32 = 0.;

        let mut glyphs: Vec<LayoutGlyph> = vec![];

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                total_width += glyph.w;
                glyphs.push(glyph.clone());
            }
        }

        (total_width, total_height, glyphs)
    }

    pub fn fill_to_cmds(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        scale: Scale,
        position: Pos,
        justify: (f32, f32),
        config: TextConfig,
    ) -> Result<Vec<(FontColor, GlyphDrawCommands)>, ErrorKind> {
        let fs = &mut self.font_system;
        let buffer = &mut self.buffer;
        let rendered_glyphs = &mut self.rendered_glyphs;

        let mut alpha_cmd_map = HashMap::new();
        let mut color_cmd_map = HashMap::new();

        let lines = buffer.layout_runs().filter(|run| run.line_w != 0.0).count();
        let total_height = lines as f32 * buffer.metrics().line_height;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical(
                    (
                        position.x,
                        position.y + scale.height * justify.1 - total_height * justify.1,
                    ),
                    1.0,
                );
                let cache_key = physical_glyph.cache_key;

                // perform cache lookup for rendered glyph
                let Some(rendered) = rendered_glyphs.entry(cache_key).or_insert_with(|| {
                    // ...or insert it

                    // do the actual rasterization
                    let font = fs
                        .get_font(cache_key.font_id)
                        .expect("Somehow shaped a font that doesn't exist");
                    let mut scaler = self
                        .scale_context
                        .builder(font.as_swash())
                        .size(f32::from_bits(cache_key.font_size_bits))
                        .hint(config.hint)
                        .build();
                    let offset =
                        Vector::new(cache_key.x_bin.as_float(), cache_key.y_bin.as_float());
                    let image = Render::new(&[
                        Source::ColorOutline(0),
                        Source::ColorBitmap(StrikeWith::BestFit),
                        Source::Outline,
                    ])
                    .format(if config.subpixel {
                        Format::Subpixel
                    } else {
                        Format::Alpha
                    })
                    .offset(offset)
                    .render(&mut scaler, cache_key.glyph_id);

                    // upload it to the GPU
                    image.map(|image| {
                        // pick an atlas texture for our glyph
                        let content_w = image.placement.width as usize;
                        let content_h = image.placement.height as usize;
                        let alloc_w = image.placement.width + (GLYPH_MARGIN + GLYPH_PADDING) * 2;
                        let alloc_h = image.placement.height + (GLYPH_MARGIN + GLYPH_PADDING) * 2;
                        let used_w = image.placement.width + GLYPH_PADDING * 2;
                        let used_h = image.placement.height + GLYPH_PADDING * 2;
                        let mut found = None;
                        for (texture_index, glyph_atlas) in
                            self.glyph_textures.iter_mut().enumerate()
                        {
                            if let Some((x, y)) = glyph_atlas
                                .atlas
                                .add_rect(alloc_w as usize, alloc_h as usize)
                            {
                                found = Some((texture_index, x, y));
                                break;
                            }
                        }
                        let (texture_index, atlas_alloc_x, atlas_alloc_y) =
                            found.unwrap_or_else(|| {
                                // if no atlas could fit the texture, make a new atlas tyvm
                                // TODO error handling
                                let mut atlas = Atlas::new(TEXTURE_SIZE, TEXTURE_SIZE);
                                let image_id = canvas
                                    .create_image(
                                        Img::new(
                                            vec![
                                                RGBA8::new(0, 0, 0, 0);
                                                TEXTURE_SIZE * TEXTURE_SIZE
                                            ],
                                            TEXTURE_SIZE,
                                            TEXTURE_SIZE,
                                        )
                                        .as_ref(),
                                        ImageFlags::empty(),
                                    )
                                    .unwrap();
                                let texture_index = self.glyph_textures.len();
                                let (x, y) =
                                    atlas.add_rect(alloc_w as usize, alloc_h as usize).unwrap();
                                self.glyph_textures.push(FontTexture { atlas, image_id });
                                (texture_index, x, y)
                            });

                        let atlas_used_x = atlas_alloc_x as u32 + GLYPH_MARGIN;
                        let atlas_used_y = atlas_alloc_y as u32 + GLYPH_MARGIN;
                        let atlas_content_x = atlas_alloc_x as u32 + GLYPH_MARGIN + GLYPH_PADDING;
                        let atlas_content_y = atlas_alloc_y as u32 + GLYPH_MARGIN + GLYPH_PADDING;

                        let mut src_buf = Vec::with_capacity(content_w * content_h);
                        match image.content {
                            Content::Mask => {
                                for chunk in image.data.chunks_exact(1) {
                                    src_buf.push(RGBA8::new(chunk[0], 0, 0, 0));
                                }
                            }
                            Content::Color | Content::SubpixelMask => {
                                for chunk in image.data.chunks_exact(4) {
                                    src_buf
                                        .push(RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]));
                                }
                            }
                        }
                        canvas
                            .update_image::<ImageSource>(
                                self.glyph_textures[texture_index].image_id,
                                ImgRef::new(&src_buf, content_w, content_h).into(),
                                atlas_content_x as usize,
                                atlas_content_y as usize,
                            )
                            .unwrap();
                        RenderedGlyph {
                            texture_index,
                            width: used_w,
                            height: used_h,
                            offset_x: image.placement.left,
                            offset_y: image.placement.top,
                            atlas_x: atlas_used_x,
                            atlas_y: atlas_used_y,
                            color_glyph: matches!(image.content, Content::Color),
                        }
                    })
                }) else {
                    continue;
                };

                let cmd_map = if rendered.color_glyph {
                    &mut color_cmd_map
                } else {
                    alpha_cmd_map
                        .entry(glyph.color_opt.unwrap_or(FontColor::rgb(0, 0, 0)))
                        .or_insert_with(HashMap::default)
                };

                let cmd = cmd_map
                    .entry(rendered.texture_index)
                    .or_insert_with(|| DrawCommand {
                        image_id: self.glyph_textures[rendered.texture_index].image_id,
                        quads: Vec::new(),
                    });

                let mut q = Quad::default();
                let it = 1.0 / TEXTURE_SIZE as f32;
                q.x0 = (physical_glyph.x + rendered.offset_x - GLYPH_PADDING as i32) as f32;
                q.y0 = (physical_glyph.y - rendered.offset_y - GLYPH_PADDING as i32
                    + run.line_y.round() as i32) as f32;
                q.x1 = q.x0 + rendered.width as f32;
                q.y1 = q.y0 + rendered.height as f32;

                q.s0 = rendered.atlas_x as f32 * it;
                q.t0 = rendered.atlas_y as f32 * it;
                q.s1 = (rendered.atlas_x + rendered.width) as f32 * it;
                q.t1 = (rendered.atlas_y + rendered.height) as f32 * it;

                cmd.quads.push(q);
            }
        }

        if !alpha_cmd_map.is_empty() {
            Ok(alpha_cmd_map
                .into_iter()
                .map(|(color, map)| {
                    (
                        color,
                        GlyphDrawCommands {
                            alpha_glyphs: map.into_values().collect(),
                            color_glyphs: color_cmd_map.drain().map(|(_, cmd)| cmd).collect(),
                        },
                    )
                })
                .collect())
        } else {
            Ok(vec![(
                FontColor(0),
                GlyphDrawCommands {
                    alpha_glyphs: vec![],
                    color_glyphs: color_cmd_map.drain().map(|(_, cmd)| cmd).collect(),
                },
            )])
        }
    }
}
