use std::mem::{replace, take};

use flagset::{flags, FlagSet};
use pdf::{
    content::Operation as PdfOperation,
    primitive::{Dictionary, PdfString, Primitive},
};

pub enum Operator {
    GraphicsState(GraphicsStateOperator),
    PathConstruction(PathConstructionOperator),
    PathPainting(PathPaintingOperator),
    ClippingPath(ClippingPathOperator),
    TextObject(TextObjectOperator),
    TextState(TextStateOperator),
    TextPositioning(TextPositioningOperator),
    TextShowing(TextShowingOperator),
    Colour(ColourOperatorPair),
    MarkedContent(MarkedContentOperator),
    Unknown(PdfOperation),
}

pub enum GraphicsStateOperator {
    // q
    Push,
    // Q
    Pop,
    // w
    SetLineWidth(UserSpaceUnit),
}

pub enum PathConstructionOperator {
    // re
    AppendRectangle(AppendRectangle),
}

pub struct AppendRectangle {
    pub x: UserSpaceUnit,
    pub y: UserSpaceUnit,
    pub width: UserSpaceUnit,
    pub height: UserSpaceUnit,
}

pub enum PathPaintingOperator {
    // n
    EndPath,
}

pub enum ClippingPathOperator {
    // W
    AppendNonzero,
}

pub enum TextObjectOperator {
    // BT
    Begin,
    // ET
    End,
}

pub enum TextStateOperator {
    // Tc
    SetCharacterSpacing(UnscaledTextSpaceUnit),
    // Tw
    SetWordSpacing(UnscaledTextSpaceUnit),
    // Tf
    SetTextFont(SetTextFont),
    // Tr
    SetRenderingMode(FlagSet<TextRenderingMode>),
}

pub struct SetTextFont {
    pub font: FontKey,
    pub size: PdfNumber,
}

pub struct FontKey(NameObject);

flags! {
    pub enum TextRenderingMode: u8 {
        Fill,
        Stroke,
        Clip,
    }
}

pub enum TextPositioningOperator {
    // Td / TD / T*
    NextLine(Option<NextLine>),
    // Tm
    SetTextMatrix(TextMatrix),
}

pub struct NextLine {
    pub tx: UnscaledTextSpaceUnit,
    pub ty: UnscaledTextSpaceUnit,
    pub set_leading_param: bool,
}

pub struct TextMatrix {
    pub a: UnscaledTextSpaceUnit,
    pub b: UnscaledTextSpaceUnit,
    pub c: UnscaledTextSpaceUnit,
    pub d: UnscaledTextSpaceUnit,
    pub e: UnscaledTextSpaceUnit,
    pub f: UnscaledTextSpaceUnit,
}

pub enum TextShowingOperator {
    // Tj
    Show(PdfString),
    // TJ
    Adjusted(Vec<TextShowingElement>),
}

pub enum TextShowingElement {
    String(PdfString),
    TextSpace(ThousandsTextSpaceUnit),
}

pub struct ColourOperatorPair {
    pub operator: ColourOperator,
    pub object: ColourObject,
}

pub enum ColourOperator {
    // CS / cs
    SetCurrentColourSpace(ColourSpace),
    // SCN / scn
    SetColour(SetColour),
}

// TODO this should be an enum depending on it directly specifies the name or is a key of
// ColorSpace subdictionary
pub struct ColourSpace(String);

// TODO check appropriate color
pub struct SetColour(Vec<Primitive>);

pub enum ColourObject {
    Stroking,
    Nonstroking,
}

pub enum MarkedContentOperator {
    // BMC/BDC
    Begin(BeginMarkedContent),
    // EMC
    End,
}

pub struct BeginMarkedContent {
    pub tag: NameObject,
    // present only in BDC (not in BMC)
    pub properties: Option<MarkedContentProperties>,
}

pub enum MarkedContentProperties {
    Dictionary(Dictionary),
    PropertiesKey(NameObject),
}

pub struct NameObject(String);

pub struct UserSpaceUnit(PdfNumber);
pub struct UnscaledTextSpaceUnit(PdfNumber);
pub struct ThousandsTextSpaceUnit(PdfNumber);

pub enum PdfNumber {
    Integer(i32),
    Number(f32),
}

impl TryFrom<PdfOperation> for Operator {
    type Error = OperatorParseError;

    fn try_from(mut operation: PdfOperation) -> Result<Self, Self::Error> {
        macro_rules! pdf_number {
            ($e: expr) => {
                match (&*$e).try_into() {
                    Ok(v) => v,
                    Err(reason) => return Err(OperatorParseError { reason }),
                }
            };
        }
        let ret = match (operation.operator.as_ref(), &mut operation.operands[..]) {
            ("q", []) => Operator::GraphicsState(GraphicsStateOperator::Push),
            ("Q", []) => Operator::GraphicsState(GraphicsStateOperator::Pop),
            ("w", [line_width]) => Operator::GraphicsState(GraphicsStateOperator::SetLineWidth(
                UserSpaceUnit(pdf_number!(line_width)),
            )),
            ("re", [x, y, width, height]) => Operator::PathConstruction({
                PathConstructionOperator::AppendRectangle(AppendRectangle {
                    x: UserSpaceUnit(pdf_number!(x)),
                    y: UserSpaceUnit(pdf_number!(y)),
                    width: UserSpaceUnit(pdf_number!(width)),
                    height: UserSpaceUnit(pdf_number!(height)),
                })
            }),
            ("n", []) => Operator::PathPainting(PathPaintingOperator::EndPath),
            ("W", []) => Operator::ClippingPath(ClippingPathOperator::AppendNonzero),
            ("BT", []) => Operator::TextObject(TextObjectOperator::Begin),
            ("ET", []) => Operator::TextObject(TextObjectOperator::End),
            ("Tc", [v]) => Operator::TextState(TextStateOperator::SetCharacterSpacing(
                UnscaledTextSpaceUnit(pdf_number!(v)),
            )),
            ("Tw", [v]) => Operator::TextState(TextStateOperator::SetWordSpacing(
                UnscaledTextSpaceUnit(pdf_number!(v)),
            )),
            ("Tf", [Primitive::Name(name), size]) => {
                Operator::TextState(TextStateOperator::SetTextFont(SetTextFont {
                    font: FontKey(NameObject(name.to_owned())),
                    size: pdf_number!(size),
                }))
            }
            ("Tr", [Primitive::Integer(mode)]) => {
                use TextRenderingMode::*;
                let mode = match *mode {
                    0 => Fill.into(),
                    1 => Stroke.into(),
                    2 => Fill | Stroke,
                    3 => FlagSet::default(),
                    4 => Fill | Clip,
                    5 => Stroke | Clip,
                    6 => Fill | Stroke | Clip,
                    7 => Clip.into(),
                    _ => {
                        return Err(OperatorParseError {
                            reason: format!("Invalid text rendering mode: {mode}"),
                        })
                    }
                };
                Operator::TextState(TextStateOperator::SetRenderingMode(mode))
            }
            ("Td" | "TD", [tx, ty]) => {
                Operator::TextPositioning(TextPositioningOperator::NextLine(Some(NextLine {
                    set_leading_param: operation.operator == "TD",
                    tx: UnscaledTextSpaceUnit(pdf_number!(tx)),
                    ty: UnscaledTextSpaceUnit(pdf_number!(ty)),
                })))
            }
            ("T*", []) => Operator::TextPositioning(TextPositioningOperator::NextLine(None)),
            ("Tm", [a, b, c, d, e, f]) => {
                Operator::TextPositioning(TextPositioningOperator::SetTextMatrix(TextMatrix {
                    a: UnscaledTextSpaceUnit(pdf_number!(a)),
                    b: UnscaledTextSpaceUnit(pdf_number!(b)),
                    c: UnscaledTextSpaceUnit(pdf_number!(c)),
                    d: UnscaledTextSpaceUnit(pdf_number!(d)),
                    e: UnscaledTextSpaceUnit(pdf_number!(e)),
                    f: UnscaledTextSpaceUnit(pdf_number!(f)),
                }))
            }
            ("Tj", [Primitive::String(s)]) => Operator::TextShowing(TextShowingOperator::Show(
                replace(s, PdfString::new(vec![])),
            )),
            ("TJ", [Primitive::Array(a)]) => Operator::TextShowing(TextShowingOperator::Adjusted(
                take(a)
                    .into_iter()
                    .map(|p| {
                        Ok(match p {
                            Primitive::String(s) => TextShowingElement::String(s),
                            _ => TextShowingElement::TextSpace(ThousandsTextSpaceUnit(
                                pdf_number!(&p),
                            )),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            (op @ ("CS" | "cs"), [colour_space]) => Operator::Colour(ColourOperatorPair {
                operator: ColourOperator::SetCurrentColourSpace(match colour_space {
                    Primitive::Name(s) => ColourSpace(take(s)),
                    _ => {
                        return Err(OperatorParseError {
                            reason: format!("Invalid colour space parameter {colour_space:?}"),
                        })
                    }
                }),
                object: match op {
                    "CS" => ColourObject::Stroking,
                    "cs" => ColourObject::Nonstroking,
                    _ => unreachable!(),
                },
            }),
            ("SCN", _) => Operator::Colour(ColourOperatorPair {
                operator: ColourOperator::SetColour(SetColour(operation.operands)),
                object: ColourObject::Stroking,
            }),
            ("scn", _) => Operator::Colour(ColourOperatorPair {
                operator: ColourOperator::SetColour(SetColour(operation.operands)),
                object: ColourObject::Stroking,
            }),
            ("BDC", [Primitive::Name(tag), properties]) => {
                Operator::MarkedContent(MarkedContentOperator::Begin(BeginMarkedContent {
                    tag: NameObject(take(tag)),
                    properties: Some(match properties {
                        Primitive::Dictionary(d) => MarkedContentProperties::Dictionary(take(d)),
                        Primitive::Name(n) => {
                            MarkedContentProperties::PropertiesKey(NameObject(take(n)))
                        }
                        _ => {
                            return Err(OperatorParseError {
                                reason: format!(
                                    "Invalid properties for a marked content: {properties:?}"
                                ),
                            })
                        }
                    }),
                }))
            }
            ("EMC", []) => Operator::MarkedContent(MarkedContentOperator::End),
            (
                "q" | "Q" | "w" | "re" | "n" | "W" | "BT" | "ET" | "Tc" | "Tw" | "Tf" | "Tr" | "Td"
                | "TD" | "T*" | "Tm" | "Tj" | "TJ" | "CS" | "cs" | "BDC" | "EMC",
                _,
            ) => {
                return Err(OperatorParseError {
                    reason: format!("Invalid number of arguments: {operation:?}"),
                })
            }
            _ => Operator::Unknown(operation),
        };
        Ok(ret)
    }
}
impl TryFrom<&Primitive> for PdfNumber {
    type Error = String;

    fn try_from(value: &Primitive) -> Result<Self, Self::Error> {
        match *value {
            Primitive::Integer(v) => Ok(PdfNumber::Integer(v)),
            Primitive::Number(v) => Ok(PdfNumber::Number(v)),
            _ => Err(format!(
                "Failed to parse PDF number: expected integer or number, found {:?}",
                value
            )),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{reason}")]
pub struct OperatorParseError {
    pub reason: String,
}
