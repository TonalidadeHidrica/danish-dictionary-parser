use flagset::flags;
use pdf::primitive::{Dictionary, PdfString, Primitive};

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
    MarkedContent(MarkedContent),
    Unknown(pdf::content::Operation),
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
    SetRenderingMode(TextRenderingMode),
}

pub struct SetTextFont {
    pub font: FontKey,
    pub size: PdfNumber,
}

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
    SetCurrentColourSpace(ColourSpace),
    SetColour(SetColour),
}

// TODO this shuold be an enum depending on it directly specifies the name or is a key of
// ColorSpace subdictionary
pub struct ColourSpace(String);

pub struct SetColour(Vec<Primitive>);

pub enum ColourObject {
    Stroking,
    Nonstroking,
}

pub enum MarkedContent {
    // BMC/BDC
    BeginMarkedContent(BeginMarkedContent),
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

pub struct FontKey(String);
