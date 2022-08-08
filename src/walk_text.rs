use std::fmt::Debug;

use anyhow::{anyhow, Context};

use pdf::{
    content::{Op, TextDrawAdjusted},
    object::PageRc,
    primitive::{Name, PdfString},
};

use crate::text_parser::{TextMatrices, TextStateParams};

pub struct ForEachText {
    operations: std::vec::IntoIter<Op>,
    params: TextStateParams,
    positions: Option<TextMatrices>,
    text_draw_adjusted_array: Option<(TextMatrices, Name, std::vec::IntoIter<TextDrawAdjusted>)>,
}

pub fn each_text(file: &pdf::file::File<Vec<u8>>, page: &PageRc) -> anyhow::Result<ForEachText> {
    let contents = page
        .contents
        .as_ref()
        .context("The page does not have contents")?;
    let params = TextStateParams::default();
    let positions = None;
    let operations = contents.operations(file)?;
    Ok(ForEachText {
        operations: operations.into_iter(),
        params,
        positions,
        text_draw_adjusted_array: None,
    })
}

impl Iterator for ForEachText {
    type Item = anyhow::Result<TextEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((positions, font, array)) = self.text_draw_adjusted_array.as_mut() {
            for a in array {
                if let TextDrawAdjusted::Text(text) = a {
                    return Some(Ok(TextEntry {
                        positions: *positions,
                        font: font.clone(),
                        text,
                    }));
                }
            }
            self.text_draw_adjusted_array = None;
        }
        for op in &mut self.operations {
            match op {
                Op::CharSpacing { char_space } => {
                    self.params.set_character_spacing(char_space);
                }
                Op::WordSpacing { word_space } => {
                    self.params.set_word_spacing(word_space);
                }
                Op::TextScaling { horiz_scale } => {
                    self.params.set_horizontal_scaling(horiz_scale);
                }
                Op::Leading { leading } => {
                    self.params.set_leading(leading);
                }
                Op::TextFont { name, size } => {
                    self.params.set_font(name, size);
                }
                Op::TextRenderMode { mode } => {
                    self.params.set_rendering_mode(mode);
                }
                Op::TextRise { rise } => {
                    self.params.set_rise(rise);
                }
                Op::BeginText => {
                    self.positions = Some(TextMatrices::default());
                }
                Op::EndText => {
                    self.positions = None;
                }
                Op::MoveTextPosition { translation } => match self.positions.as_mut() {
                    None => return Some(Err(anyhow!("BT not present before Td/TD"))),
                    Some(e) => e.next_line(translation),
                },
                Op::SetTextMatrix { matrix } => match self.positions.as_mut() {
                    None => return Some(Err(anyhow!("BT not present before Tm"))),
                    Some(e) => e.set_matrix(matrix),
                },
                Op::TextDraw { text } => {
                    let positions = match self.positions {
                        None => return Some(Err(anyhow!("BT not preset beefore Tj"))),
                        Some(e) => e,
                    };
                    let (font, _) = match self.params.font().clone() {
                        None => return Some(Err(anyhow!("Tf not present before Tj"))),
                        Some(e) => e,
                    };
                    return Some(Ok(TextEntry {
                        positions,
                        font,
                        text,
                    }));
                }
                Op::TextDrawAdjusted { array } => {
                    let positions = match self.positions {
                        None => return Some(Err(anyhow!("BT not preset beefore Tj"))),
                        Some(e) => e,
                    };
                    let (font, _) = match self.params.font().clone() {
                        None => return Some(Err(anyhow!("Tf not present before Tj"))),
                        Some(e) => e,
                    };
                    self.text_draw_adjusted_array = Some((positions, font, array.into_iter()));
                    return self.next();
                }
                _ => {}
            }
        }
        None
    }
}

pub struct TextEntry {
    pub positions: TextMatrices,
    pub font: Name,
    pub text: PdfString,
}

impl Debug for TextEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEntry")
            .field("pos", &self.positions.coordinates())
            .field("font", &self.font)
            .field("size", &self.positions.glyph_size())
            .field("text", &self.text)
            .finish()
    }
}
