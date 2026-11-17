//! Inherited typography and nested styled text descriptions.
use super::{Paragraph, TextOverflow, TextSpan};
use crate::{
    element::{BuildContext, DependencyError, StatelessWidget, Widget},
    layout::LayoutError,
};
use egui::{Align, Color32, FontFamily, FontId, Stroke, text::TextFormat};

/// A line height override can explicitly restore font metrics in a nested style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Normal,
    Points(f32),
}
/// Unspecified properties inherit. `inherit = false` replaces the receiver in
/// `merge`; unspecified replacement properties use the framework fallback.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub inherit: bool,
    pub font_size: Option<f32>,
    pub font_family: Option<FontFamily>,
    pub color: Option<Color32>,
    pub background: Option<Color32>,
    pub letter_spacing: Option<f32>,
    pub line_height: Option<LineHeight>,
    pub italics: Option<bool>,
    pub underline: Option<Stroke>,
    pub strikethrough: Option<Stroke>,
    pub vertical_align: Option<Align>,
    pub background_expansion: Option<f32>,
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            inherit: true,
            font_size: None,
            font_family: None,
            color: None,
            background: None,
            letter_spacing: None,
            line_height: None,
            italics: None,
            underline: None,
            strikethrough: None,
            vertical_align: None,
            background_expansion: None,
        }
    }
}
impl TextStyle {
    /// Overlay specified properties. Replacement semantics depend on `other`.
    pub fn merge(&self, other: &Self) -> Self {
        if !other.inherit {
            return other.clone();
        }
        Self {
            inherit: self.inherit,
            font_size: other.font_size.or(self.font_size),
            font_family: other
                .font_family
                .clone()
                .or_else(|| self.font_family.clone()),
            color: other.color.or(self.color),
            background: other.background.or(self.background),
            letter_spacing: other.letter_spacing.or(self.letter_spacing),
            line_height: other.line_height.or(self.line_height),
            italics: other.italics.or(self.italics),
            underline: other.underline.or(self.underline),
            strikethrough: other.strikethrough.or(self.strikethrough),
            vertical_align: other.vertical_align.or(self.vertical_align),
            background_expansion: other.background_expansion.or(self.background_expansion),
        }
    }
    /// Fill remaining properties with 14-point proportional white text and
    /// ordinary backend geometry. Invalid numeric values fail paragraph layout.
    pub fn to_format(&self) -> TextFormat {
        TextFormat {
            font_id: FontId::new(
                self.font_size.unwrap_or(14.0),
                self.font_family.clone().unwrap_or(FontFamily::Proportional),
            ),
            color: self.color.unwrap_or(Color32::WHITE),
            background: self.background.unwrap_or(Color32::TRANSPARENT),
            extra_letter_spacing: self.letter_spacing.unwrap_or(0.0),
            line_height: match self.line_height {
                Some(LineHeight::Points(height)) => Some(height),
                _ => None,
            },
            italics: self.italics.unwrap_or(false),
            underline: self.underline.unwrap_or(Stroke::NONE),
            strikethrough: self.strikethrough.unwrap_or(Stroke::NONE),
            valign: self.vertical_align.unwrap_or(Align::Max),
            expand_bg: self.background_expansion.unwrap_or(1.0),
        }
    }
}
/// Explicit positive linear text scaling. This does not read OS accessibility
/// settings and does not implement a nonlinear font-scaling strategy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextScale(f32);
impl Default for TextScale {
    fn default() -> Self {
        Self(1.0)
    }
}
impl TextScale {
    pub fn linear(factor: f32) -> Result<Self, LayoutError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(LayoutError::InvalidConfiguration(
                "text scale must be finite and positive",
            ));
        }
        Ok(Self(factor))
    }
    pub fn factor(self) -> f32 {
        self.0
    }
    fn apply(self, mut format: TextFormat) -> TextFormat {
        format.font_id.size *= self.0;
        format.line_height = format.line_height.map(|height| height * self.0);
        format
    }
}
/// Closest scoped defaults for high-level Text widgets. Providing a value
/// replaces the outer defaults. Use `merged_style` to overlay just typography.
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultTextStyle {
    pub style: TextStyle,
    pub align: Align,
    pub soft_wrap: bool,
    pub max_lines: Option<usize>,
    pub overflow: TextOverflow,
    pub scale: TextScale,
}
impl Default for DefaultTextStyle {
    fn default() -> Self {
        Self {
            style: TextStyle::default(),
            align: Align::Min,
            soft_wrap: true,
            max_lines: None,
            overflow: TextOverflow::Clip,
            scale: TextScale::default(),
        }
    }
}
impl DefaultTextStyle {
    /// Listen to the nearest defaults. Like other inherited dependencies, this
    /// may be called during build/dependency hooks, not component initialization.
    pub fn of(ctx: &BuildContext) -> Result<Self, DependencyError> {
        Ok(ctx
            .depend_on::<Self>()?
            .map(|value| (*value).clone())
            .unwrap_or_default())
    }
    pub fn provide(self, child: Widget) -> Widget {
        Widget::inherited(self, child)
    }
    pub fn merged_style(&self, style: &TextStyle) -> Self {
        Self {
            style: self.style.merge(style),
            ..self.clone()
        }
    }
}
/// Nested inline text. A node emits its own optional text before its children.
/// Span inheritance always fills unspecified properties from the parent span;
/// the `TextStyle::inherit` flag only controls ordinary style merge operations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InlineSpan {
    pub text: Option<String>,
    pub style: TextStyle,
    pub children: Vec<InlineSpan>,
}
impl InlineSpan {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }
    pub fn group(children: Vec<Self>) -> Self {
        Self {
            children,
            ..Self::default()
        }
    }
    pub fn styled(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
    fn flatten(&self, base: &TextStyle, scale: TextScale) -> Vec<TextSpan> {
        let mut spans = vec![];
        let mut pending = vec![(self, base.clone())];
        while let Some((span, parent)) = pending.pop() {
            let mut overlay = span.style.clone();
            // Unlike Text.style, inline inheritance ignores the inherit flag.
            overlay.inherit = true;
            let resolved = parent.merge(&overlay);
            if let Some(text) = &span.text {
                spans.push(TextSpan::new(
                    text.clone(),
                    scale.apply(resolved.to_format()),
                ));
            }
            for child in span.children.iter().rev() {
                pending.push((child, resolved.clone()));
            }
        }
        spans
    }
}
/// Explicitly distinguish inheriting a line limit from removing one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineLimit {
    #[default]
    Inherit,
    Unlimited,
    Lines(usize),
}
/// High-level text component. Style and layout defaults come from the closest
/// DefaultTextStyle. Render-object identity is retained across inherited changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub content: InlineSpan,
    pub style: TextStyle,
    pub align: Option<Align>,
    pub soft_wrap: Option<bool>,
    pub line_limit: LineLimit,
    pub overflow: Option<TextOverflow>,
    pub scale: Option<TextScale>,
}
impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self::rich(InlineSpan::new(text))
    }
    pub fn rich(content: InlineSpan) -> Self {
        Self {
            content,
            style: TextStyle::default(),
            align: None,
            soft_wrap: None,
            line_limit: LineLimit::Inherit,
            overflow: None,
            scale: None,
        }
    }
    pub fn into_widget(self) -> Widget {
        Widget::stateless(self)
    }
    /// Pure style resolution. Invalid final geometry is reported when the
    /// resulting Paragraph is measured, just like directly configured paragraphs.
    pub fn resolve(&self, defaults: &DefaultTextStyle) -> Paragraph {
        let style = defaults.style.merge(&self.style);
        let mut paragraph = Paragraph::rich(
            self.content
                .flatten(&style, self.scale.unwrap_or(defaults.scale)),
        );
        paragraph.align = self.align.unwrap_or(defaults.align);
        paragraph.soft_wrap = self.soft_wrap.unwrap_or(defaults.soft_wrap);
        paragraph.max_lines = match self.line_limit {
            LineLimit::Inherit => defaults.max_lines,
            LineLimit::Unlimited => None,
            LineLimit::Lines(count) => Some(count),
        };
        paragraph.overflow = self.overflow.unwrap_or(defaults.overflow);
        paragraph
    }
}
impl StatelessWidget for Text {
    fn build(&self, ctx: &BuildContext) -> Widget {
        self.resolve(&DefaultTextStyle::of(ctx).expect("Text resolves defaults during build"))
            .into_widget()
    }
}
