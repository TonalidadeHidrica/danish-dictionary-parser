use getset::Setters;
use nalgebra::Matrix3;
use pdf::{content::TextMode, primitive::Name};

#[derive(Setters)]
pub struct TextStateParams {
    #[getset(set = "pub")]
    character_spacing: f32,
    #[getset(set = "pub")]
    word_spacing: f32,
    #[getset(set = "pub")]
    horizontal_scaling: f32,
    #[getset(set = "pub")]
    leading: f32,
    font: Option<(Name, f32)>,
    #[getset(set = "pub")]
    rendering_mode: TextMode,
    #[getset(set = "pub")]
    rise: f32,
}
impl Default for TextStateParams {
    fn default() -> Self {
        Self {
            character_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            font: None,
            rendering_mode: TextMode::Fill,
            rise: 0.0,
        }
    }
}

impl TextStateParams {
    pub fn set_font(&mut self, font: Name, size: f32) {
        self.font = Some((font, size));
    }
}

pub struct TextMatrices {
    text_matrix: Matrix3<f32>,
    text_line_matrix: Matrix3<f32>,
}
impl Default for TextMatrices {
    fn default() -> Self {
        Self {
            text_matrix: Matrix3::identity(),
            text_line_matrix: Matrix3::identity(),
        }
    }
}
impl TextMatrices {
    pub fn next_line(&mut self, t: pdf::content::Point) {
        self.text_line_matrix =
            Matrix3::new(1., 0., 0., 0., 1., 0., t.x, t.y, 1.) * self.text_line_matrix;
        self.text_matrix = self.text_line_matrix;
    }
    pub fn set_matrix(&mut self, m: pdf::content::Matrix) {
        self.text_line_matrix = Matrix3::new(m.a, m.b, 0., m.c, m.d, 0., m.e, m.f, 1.);
        self.text_matrix = self.text_line_matrix;
    }
}
