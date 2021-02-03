use failure::{self, Error};
use std::slice;
use unicode_width::UnicodeWidthStr;

pub mod ftwrap;
pub mod hbwrap;
pub mod fcwrap;

pub use self::fcwrap::Pattern as FontPattern;

#[derive(Clone, Debug)]
pub struct GlyphInfo {
    /// We only retain text in debug mode for diagnostic purposes
    #[cfg(debug_assertions)]
    pub text: String,
    pub num_cells: u8,
    pub font_idx: usize,
    pub glyph_pos: u32,
    pub cluster: u32,
    pub x_advance: i32,
    pub y_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

impl GlyphInfo {
    pub fn new(
        text: &str,
        font_idx: usize,
        info: &hbwrap::hb_glyph_info_t,
        pos: &hbwrap::hb_glyph_position_t,
    ) -> GlyphInfo {
        let num_cells = UnicodeWidthStr::width(text) as u8;
        GlyphInfo {
            #[cfg(debug_assertions)]
            text: text.into(),
            num_cells,
            font_idx,
            glyph_pos: info.codepoint,
            cluster: info.cluster,
            x_advance: pos.x_advance / 64,
            y_advance: pos.y_advance / 64,
            x_offset: pos.x_offset / 64,
            y_offset: pos.y_offset / 64,
        }
    }
}

struct FontInfo {
    face: ftwrap::Face,
    font: hbwrap::Font,
    cell_height: i64,
    cell_width: i64,
}

pub struct Font {
    lib: ftwrap::Library,
    pattern: fcwrap::Pattern,
    font_list: fcwrap::FontSet,
    fonts: Vec<FontInfo>,
}

impl Drop for Font {
    fn drop(&mut self) {
        // Ensure that we drop the fonts before we drop the
        // library, otherwise we will end up faulting
        self.fonts.clear();
    }
}

impl Font {
    pub fn new(mut pattern: FontPattern) -> Result<Font, Error> {
        let mut lib = ftwrap::Library::new()?;
        lib.set_lcd_filter(
            ftwrap::FT_LcdFilter::FT_LCD_FILTER_DEFAULT,
        )?;

        //pattern.family("Operator Mono SSm")?;
        pattern.monospace()?;
        pattern.config_substitute(fcwrap::MatchKind::Pattern)?;
        pattern.default_substitute();
        let font_list = pattern.sort(true)?;

        Ok(Font {
            lib,
            font_list,
            pattern,
            fonts: Vec::new(),
        })
    }

    fn load_next_fallback(&mut self) -> Result<(), Error> {
        let idx = self.fonts.len();
        let pat = self.font_list.iter().nth(idx).ok_or(failure::err_msg(
            "no more fallbacks",
        ))?;
        let pat = self.pattern.render_prepare(&pat)?;
        let file = pat.get_file()?;

        println!("load_next_fallback: file={}", file);

        let size = pat.get_double("size")?.ceil() as i64;

        let mut face = self.lib.new_face(file, 0)?;
        match face.set_char_size(0, size * 64, 96, 96) {
            Err(err) => {
                let sizes = unsafe {
                    let rec = &(*face.face);
                    slice::from_raw_parts(rec.available_sizes, rec.num_fixed_sizes as usize)
                };
                if sizes.len() == 0 {
                    return Err(err);
                } else {
                    // Find the best matching size.
                    // We just take the biggest.
                    let mut size = 0i16;
                    for info in sizes.iter() {
                        size = size.max(info.height);
                    }
                    face.set_pixel_sizes(size as u32, size as u32)?;
                }
            }
            Ok(_) => {}
        }
        let font = hbwrap::Font::new(&face);

        // Compute metrics for the nominal monospace cell
        let (cell_width, cell_height) = face.cell_metrics();
        println!("metrics: width={} height={}", cell_width, cell_height);

        self.fonts.push(FontInfo {
            face,
            font,
            cell_height,
            cell_width,
        });
        Ok(())
    }

    fn get_font(&mut self, idx: usize) -> Result<&mut FontInfo, Error> {
        if idx >= self.fonts.len() {
            self.load_next_fallback()?;
            ensure!(
                idx < self.fonts.len(),
                "should not ask for a font later than the next prepared font"
            );
        }

        Ok(&mut self.fonts[idx])
    }

    pub fn get_metrics(&mut self) -> Result<(i64, i64), Error> {
        let font = self.get_font(0)?;
        Ok((font.cell_height, font.cell_width))
    }

    pub fn shape(&mut self, font_idx: usize, s: &str) -> Result<Vec<GlyphInfo>, Error> {
        println!(
            "shape text for font_idx {} with len {} {}",
            font_idx,
            s.len(),
            s
        );
        let features = vec![
            // kerning
            hbwrap::feature_from_string("kern")?,
            // ligatures
            hbwrap::feature_from_string("liga")?,
            // contextual ligatures
            hbwrap::feature_from_string("clig")?,
        ];

        let mut buf = hbwrap::Buffer::new()?;
        buf.set_script(hbwrap::HB_SCRIPT_LATIN);
        buf.set_direction(hbwrap::HB_DIRECTION_LTR);
        buf.set_language(hbwrap::language_from_string("en")?);
        buf.add_str(s);

        self.shape_with_font(font_idx, &mut buf, &features)?;
        let infos = buf.glyph_infos();
        let positions = buf.glyph_positions();

        let mut cluster = Vec::new();

        let mut last_text_pos = None;
        let mut first_fallback_pos = None;

        // Compute the lengths of the text clusters.
        // Ligatures and combining characters mean
        // that a single glyph can take the place of
        // multiple characters.  The 'cluster' member
        // of the glyph info is set to the position
        // in the input utf8 text, so we make a pass
        // over the set of clusters to look for differences
        // greater than 1 and backfill the length of
        // the corresponding text fragment.  We need
        // the fragments to properly handle fallback,
        // and they're handy to have for debugging
        // purposes too.
        let mut sizes = Vec::new();
        for (i, info) in infos.iter().enumerate() {
            let pos = info.cluster as usize;
            let mut size = 1;
            if let Some(last_pos) = last_text_pos {
                let diff = pos - last_pos;
                if diff > 1 {
                    sizes[i - 1] = diff;
                }
            } else if pos != 0 {
                size = pos;
            }
            last_text_pos = Some(pos);
            sizes.push(size);
        }
        if let Some(last_pos) = last_text_pos {
            let diff = s.len() - last_pos;
            if diff > 1 {
                let last = sizes.len() - 1;
                sizes[last] = diff;
            }
        }
        println!("sizes: {:?}", sizes);

        // Now make a second pass to determine if we need
        // to perform fallback to a later font.
        // We can determine this by looking at the codepoint.
        for (i, info) in infos.iter().enumerate() {
            let pos = info.cluster as usize;
            if info.codepoint == 0 {
                if first_fallback_pos.is_none() {
                    // Start of a run that needs fallback
                    first_fallback_pos = Some(pos);
                }
            } else if let Some(start) = first_fallback_pos {
                // End of a fallback run
                println!("range: {:?}-{:?} needs fallback", start, pos);

                let substr = &s[start..pos];
                let mut shape = self.shape(font_idx + 1, substr)?;
                cluster.append(&mut shape);

                first_fallback_pos = None;
            }
            if info.codepoint != 0 {
                let text = &s[pos..pos + sizes[i]];
                println!("glyph from `{}`", text);
                cluster.push(GlyphInfo::new(text, font_idx, info, &positions[i]));
            }
        }

        // Check to see if we started and didn't finish a
        // fallback run.
        if let Some(start) = first_fallback_pos {
            let substr = &s[start..];
            println!(
                "at end {:?}-{:?} needs fallback {}",
                start,
                s.len() - 1,
                substr,
            );
            let mut shape = self.shape(font_idx + 1, substr)?;
            cluster.append(&mut shape);
        }

        println!("shaped: {:#?}", cluster);

        Ok(cluster)
    }

    fn shape_with_font(
        &mut self,
        idx: usize,
        buf: &mut hbwrap::Buffer,
        features: &Vec<hbwrap::hb_feature_t>,
    ) -> Result<(), Error> {
        let info = self.get_font(idx)?;
        info.font.shape(buf, Some(features.as_slice()));
        Ok(())
    }

    pub fn load_glyph(
        &mut self,
        font_idx: usize,
        glyph_pos: u32,
    ) -> Result<&ftwrap::FT_GlyphSlotRec_, Error> {
        let info = &mut self.fonts[font_idx];
        info.face.load_and_render_glyph(
            glyph_pos,
            (ftwrap::FT_LOAD_COLOR) as i32,
            ftwrap::FT_Render_Mode::FT_RENDER_MODE_LCD,
        )
    }
}