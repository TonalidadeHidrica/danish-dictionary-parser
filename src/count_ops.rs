use std::collections::HashMap;

use anyhow::Context;
use itertools::Itertools;
use pdf::content::Op;

pub fn count_ops(file: &pdf::file::File<Vec<u8>>) -> Result<(), anyhow::Error> {
    let mut count = HashMap::<_, u64>::new();
    for page in file.pages().flatten() {
        let contents = page
            .contents
            .as_ref()
            .context("The page does not have contents")?;
        for op in contents.operations(file)? {
            let op = op_name_to_string(op);
            *count.entry(op).or_default() += 1;
        }
    }
    for (k, v) in count.iter().sorted_by_key(|x| x.1).rev() {
        println!("{k:25}\t{v}");
    }
    Ok(())
}

fn op_name_to_string(op: Op) -> &'static str {
    match op {
        Op::BeginMarkedContent { .. } => "BeginMarkedContent",
        Op::EndMarkedContent { .. } => "EndMarkedContent",
        Op::MarkedContentPoint { .. } => "MarkedContentPoint",
        Op::Close { .. } => "Close",
        Op::MoveTo { .. } => "MoveTo",
        Op::LineTo { .. } => "LineTo",
        Op::CurveTo { .. } => "CurveTo",
        Op::Rect { .. } => "Rect",
        Op::EndPath { .. } => "EndPath",
        Op::Stroke { .. } => "Stroke",
        Op::FillAndStroke { .. } => "FillAndStroke",
        Op::Fill { .. } => "Fill",
        Op::Shade { .. } => "Shade",
        Op::Clip { .. } => "Clip",
        Op::Save { .. } => "Save",
        Op::Restore { .. } => "Restore",
        Op::Transform { .. } => "Transform",
        Op::LineWidth { .. } => "LineWidth",
        Op::Dash { .. } => "Dash",
        Op::LineJoin { .. } => "LineJoin",
        Op::LineCap { .. } => "LineCap",
        Op::MiterLimit { .. } => "MiterLimit",
        Op::Flatness { .. } => "Flatness",
        Op::GraphicsState { .. } => "GraphicsState",
        Op::StrokeColor { .. } => "StrokeColor",
        Op::FillColor { .. } => "FillColor",
        Op::FillColorSpace { .. } => "FillColorSpace",
        Op::StrokeColorSpace { .. } => "StrokeColorSpace",
        Op::RenderingIntent { .. } => "RenderingIntent",
        Op::BeginText { .. } => "BeginText",
        Op::EndText { .. } => "EndText",
        Op::CharSpacing { .. } => "CharSpacing",
        Op::WordSpacing { .. } => "WordSpacing",
        Op::TextScaling { .. } => "TextScaling",
        Op::Leading { .. } => "Leading",
        Op::TextFont { .. } => "TextFont",
        Op::TextRenderMode { .. } => "TextRenderMode",
        Op::TextRise { .. } => "TextRise",
        Op::MoveTextPosition { .. } => "MoveTextPosition",
        Op::SetTextMatrix { .. } => "SetTextMatrix",
        Op::TextNewline { .. } => "TextNewline",
        Op::TextDraw { .. } => "TextDraw",
        Op::TextDrawAdjusted { .. } => "TextDrawAdjusted",
        Op::XObject { .. } => "XObject",
        Op::InlineImage { .. } => "InlineImage",
    }
}
