//! Inherited typography and nested styled text descriptions.
use super::{Paragraph, TextOverflow, TextSpan};
use crate::{
    element::{BuildContext, DependencyError, StatelessWidget, Widget},
    layout::LayoutError,
};
use egui::{Align, Color32, FontFamily, FontId, Stroke, text::TextFormat};

const WEIGHT_FAMILY_PREFIX: &str = "rust_flutter::weight::";

/// Numeric font weight in the conventional 100–900 range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum FontWeight {
    W100 = 100,
    W200 = 200,
    W300 = 300,
    #[default]
    W400 = 400,
    W500 = 500,
    W600 = 600,
    W700 = 700,
    W800 = 800,
    W900 = 900,
}

impl FontWeight {
    /// Return the egui named family used to register this weight variant.
    /// When it is not registered, paragraph layout falls back to `base`.
    pub fn family_variant(self, base: &FontFamily) -> FontFamily {
        FontFamily::Name(format!("{WEIGHT_FAMILY_PREFIX}{}::{base}", self as u16).into())
    }

    pub(super) fn resolve_family(family: &FontFamily, available: &[FontFamily]) -> FontFamily {
        let FontFamily::Name(name) = family else {
            return family.clone();
        };
        let Some(weighted) = name.strip_prefix(WEIGHT_FAMILY_PREFIX) else {
            return family.clone();
        };
        let Some((weight, base)) = weighted.split_once("::") else {
            return family.clone();
        };
        if !matches!(
            weight,
            "100" | "200" | "300" | "400" | "500" | "600" | "700" | "800" | "900"
        ) {
            return family.clone();
        }
        let base = match base {
            "Proportional" => FontFamily::Proportional,
            "Monospace" => FontFamily::Monospace,
            other => FontFamily::Name(other.into()),
        };
        if available.contains(family) {
            family.clone()
        } else {
            base
        }
    }
}

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
    pub font_weight: Option<FontWeight>,
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
            font_weight: None,
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
            font_weight: other.font_weight.or(self.font_weight),
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
            font_id: FontId::new(self.font_size.unwrap_or(14.0), {
                let family = self.font_family.clone().unwrap_or(FontFamily::Proportional);
                self.font_weight
                    .map_or(family.clone(), |weight| weight.family_variant(&family))
            }),
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
/// Maps authored logical font sizes to rendered font sizes. Linear scaling is
/// available through [`TextScale::linear`]; nonlinear scaling can represent
/// platform accessibility curves such as Android's large-text scaling.
#[derive(Clone, Debug, PartialEq)]
pub struct TextScale {
    linear: f32,
    curve: Option<std::sync::Arc<[(f32, f32)]>>,
}
impl Default for TextScale {
    fn default() -> Self {
        Self {
            linear: 1.0,
            curve: None,
        }
    }
}
impl TextScale {
    pub fn linear(factor: f32) -> Result<Self, LayoutError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(LayoutError::InvalidConfiguration(
                "text scale must be finite and positive",
            ));
        }
        Ok(Self {
            linear: factor,
            curve: None,
        })
    }
    /// Create a piecewise-linear scaler from `(authored_size, rendered_size)`
    /// samples. Samples must have finite, positive sizes, strictly increasing
    /// authored sizes, and nondecreasing rendered sizes. Values outside the
    /// sample range extend using the nearest segment's slope.
    pub fn nonlinear(samples: impl IntoIterator<Item = (f32, f32)>) -> Result<Self, LayoutError> {
        let samples: Vec<_> = samples.into_iter().collect();
        if samples.len() < 2
            || samples.iter().any(|(authored, rendered)| {
                !authored.is_finite()
                    || !rendered.is_finite()
                    || *authored <= 0.0
                    || *rendered <= 0.0
            })
            || samples
                .windows(2)
                .any(|pair| pair[1].0 <= pair[0].0 || pair[1].1 < pair[0].1)
        {
            return Err(LayoutError::InvalidConfiguration(
                "nonlinear text scale samples must be finite, positive, and ordered",
            ));
        }
        Ok(Self {
            linear: 1.0,
            curve: Some(samples.into()),
        })
    }
    /// Return the scale ratio at a 14-point authored font size.
    pub fn factor(&self) -> f32 {
        self.scale(14.0) / 14.0
    }
    /// Convert an authored logical font size to its rendered size.
    pub fn scale(&self, size: f32) -> f32 {
        if size == 0.0 {
            return 0.0;
        }
        let Some(samples) = &self.curve else {
            return size * self.linear;
        };
        let upper = samples.partition_point(|(authored, _)| *authored < size);
        let (left, right) = if upper == 0 {
            let (authored, rendered) = samples[0];
            return size * (rendered / authored);
        } else if upper == samples.len() {
            (samples[samples.len() - 2], samples[samples.len() - 1])
        } else {
            (samples[upper - 1], samples[upper])
        };
        let proportion = (size - left.0) / (right.0 - left.0);
        left.1 + proportion * (right.1 - left.1)
    }
    fn apply(&self, mut format: TextFormat) -> TextFormat {
        format.font_id.size = self.scale(format.font_id.size);
        format.line_height = format.line_height.map(|height| self.scale(height));
        format
    }
}
/// The current operating-system accessibility text scaler. It is refreshed by
/// the host each frame; applications can observe it during a widget build.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformTextScale(pub TextScale);
impl PlatformTextScale {
    pub fn of(ctx: &BuildContext) -> Result<TextScale, DependencyError> {
        Ok(ctx
            .depend_on::<Self>()?
            .map(|value| value.0.clone())
            .unwrap_or_default())
    }
}
impl crate::element::InheritedModel for PlatformTextScale {
    type Aspect = ();

    fn update_should_notify(&self, old: &Self) -> bool {
        self != old
    }

    fn update_should_notify_dependent(
        &self,
        old: &Self,
        _aspects: &std::collections::HashSet<Self::Aspect>,
    ) -> bool {
        self != old
    }
}
#[cfg(target_os = "android")]
pub(crate) fn set_platform_text_scale(ctx: &egui::Context, scale: TextScale) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("rust_flutter_platform_text_scale"), scale));
}
pub(crate) fn platform_text_scale(ctx: &egui::Context) -> TextScale {
    ctx.data_mut(|data| {
        data.get_temp(egui::Id::new("rust_flutter_platform_text_scale"))
            .unwrap_or_default()
    })
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
    fn flatten(
        &self,
        base: &TextStyle,
        scale: &TextScale,
        platform_scale: &TextScale,
    ) -> Vec<TextSpan> {
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
                    platform_scale.apply(scale.apply(resolved.to_format())),
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
        let mut paragraph = Paragraph::rich(self.content.flatten(
            &style,
            self.scale.as_ref().unwrap_or(&defaults.scale),
            &TextScale::default(),
        ));
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
        let mut defaults = DefaultTextStyle::of(ctx).expect("Text resolves defaults during build");
        if let Some(theme) = crate::material::ThemeData::maybe_of(ctx)
            .expect("Text resolves an optional Material theme during build")
        {
            let mut material_style = theme.text_theme.body_medium;
            material_style
                .color
                .get_or_insert(theme.color_scheme.on_surface);
            defaults.style = material_style.merge(&defaults.style);
        }
        let mut paragraph = self.resolve(&defaults);
        let platform_scale =
            PlatformTextScale::of(ctx).expect("Text resolves platform text scale during build");
        if platform_scale != TextScale::default() {
            for span in &mut paragraph.spans {
                span.format = platform_scale.apply(span.format.clone());
            }
        }
        paragraph.into_widget()
    }
}
