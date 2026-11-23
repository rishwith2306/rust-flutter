//! Retained, formatted paragraphs backed by the active egui font collection.
//! Measure and show after the first egui pass has initialized fonts.
mod style;
use crate::{
    element::{RenderBox, RenderContext, RenderObjectWidget, Widget},
    layout::{
        Baselines, BoxChildren, BoxConstraints, BoxGeometry, BoxQueries, Clip, IntrinsicDimension,
        LayoutError, TextBaseline,
    },
};
use egui::{
    Align, Color32, Context, Vec2,
    text::{Galley, LayoutJob, TextFormat},
};
use std::sync::Arc;
pub(crate) use style::platform_text_scale;
#[cfg(target_os = "android")]
pub(crate) use style::set_platform_text_scale;
pub use style::{
    DefaultTextStyle, FontWeight, InlineSpan, LineHeight, LineLimit, PlatformTextScale, Text,
    TextScale, TextStyle,
};

/// Fully resolved formatting for a contiguous UTF-8 span. No implicit style
/// inheritance: configure fonts through the host Context and each TextFormat.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub format: TextFormat,
    /// Additional logical points after each sequence of whitespace.
    pub word_spacing: f32,
}
impl TextSpan {
    pub fn new(text: impl Into<String>, format: TextFormat) -> Self {
        Self {
            text: text.into(),
            format,
            word_spacing: 0.0,
        }
    }

    /// Set additional spacing between words. Non-finite values are rejected
    /// during paragraph measurement.
    pub fn with_word_spacing(mut self, word_spacing: f32) -> Self {
        self.word_spacing = word_spacing;
        self
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
    #[default]
    Clip,
    Visible,
    /// Requires max_lines; without soft wrapping requires exactly one line.
    Ellipsis,
}
/// A paragraph's widget description. Width fits its longest laid-out row, then
/// obeys parent constraints. Explicit newlines remain breaks without soft wrap.
#[derive(Clone, Debug, PartialEq)]
pub struct Paragraph {
    pub spans: Vec<TextSpan>,
    pub soft_wrap: bool,
    pub max_lines: Option<usize>,
    pub overflow: TextOverflow,
    pub align: Align,
}
impl Paragraph {
    pub fn new(text: impl Into<String>, format: TextFormat) -> Self {
        Self::rich(vec![TextSpan::new(text, format)])
    }
    pub fn rich(spans: Vec<TextSpan>) -> Self {
        Self {
            spans,
            soft_wrap: true,
            max_lines: None,
            overflow: TextOverflow::Clip,
            align: Align::Min,
        }
    }
    pub fn into_widget(self) -> Widget {
        Widget::render_object(self, vec![])
    }
    fn validate(&self) -> Result<(), LayoutError> {
        if self.max_lines == Some(0)
            || (self.overflow == TextOverflow::Ellipsis
                && (self.max_lines.is_none() || (!self.soft_wrap && self.max_lines != Some(1))))
        {
            return Err(LayoutError::InvalidConfiguration(
                "paragraph line limit/ellipsis configuration is invalid",
            ));
        }
        for span in &self.spans {
            let f = &span.format;
            if !span.word_spacing.is_finite()
                || !f.font_id.size.is_finite()
                || f.font_id.size <= 0.0
                || !f.extra_letter_spacing.is_finite()
                || f.line_height.is_some_and(|h| !h.is_finite() || h <= 0.0)
                || !f.expand_bg.is_finite()
                || f.expand_bg < 0.0
                || [f.underline.width, f.strikethrough.width]
                    .iter()
                    .any(|w| !w.is_finite() || *w < 0.0)
            {
                return Err(LayoutError::InvalidConfiguration(
                    "paragraph format contains invalid geometry",
                ));
            }
        }
        Ok(())
    }
    fn job(&self, width: f32, limited: bool) -> LayoutJob {
        let mut job = LayoutJob::default();
        let mut spacing_after_whitespace = None;
        for span in &self.spans {
            if span.text.is_empty() {
                job.append("", 0.0, span.format.clone());
                continue;
            }
            let mut start = 0;
            let mut category = None;
            for (index, character) in span.text.char_indices() {
                let next_category = if character == '\n' {
                    2
                } else if character.is_whitespace() {
                    1
                } else {
                    0
                };
                if category.is_some_and(|current| current != next_category) {
                    Self::append_text_section(
                        &mut job,
                        &span.text[start..index],
                        category.unwrap(),
                        span,
                        &mut spacing_after_whitespace,
                    );
                    start = index;
                }
                category = Some(next_category);
            }
            if let Some(category) = category {
                Self::append_text_section(
                    &mut job,
                    &span.text[start..],
                    category,
                    span,
                    &mut spacing_after_whitespace,
                );
            }
        }
        job.halign = self.align;
        job.wrap.max_width = width;
        job.wrap.max_rows = if limited {
            self.max_lines.unwrap_or(usize::MAX)
        } else {
            usize::MAX
        };
        job.wrap.overflow_character =
            (limited && self.overflow == TextOverflow::Ellipsis).then_some('…');
        job.wrap.break_anywhere = limited && self.max_lines == Some(1);
        job
    }

    fn append_text_section(
        job: &mut LayoutJob,
        text: &str,
        category: u8,
        span: &TextSpan,
        spacing_after_whitespace: &mut Option<f32>,
    ) {
        let leading_space = if category == 0 {
            spacing_after_whitespace.take().unwrap_or(0.0)
        } else {
            0.0
        };
        job.append(text, leading_space, span.format.clone());
        match category {
            1 => *spacing_after_whitespace = Some(span.word_spacing),
            2 => *spacing_after_whitespace = None,
            _ => {}
        }
    }
}
/// Persistent paragraph render object. Pure queries may populate egui's font
/// cache but never replace the galley selected by normal layout for painting.
pub struct ParagraphRender {
    paragraph: Paragraph,
    context: Context,
    painted: Option<Arc<Galley>>,
}
impl RenderObjectWidget for Paragraph {
    type Object = ParagraphRender;
    fn create_render_object(&self, ctx: &RenderContext) -> ParagraphRender {
        ParagraphRender {
            paragraph: self.clone(),
            context: ctx.egui().clone(),
            painted: None,
        }
    }
    fn update_render_object(&self, object: &mut ParagraphRender, _: &RenderContext) {
        object.paragraph = self.clone();
        object.painted = None;
    }
}
impl ParagraphRender {
    fn shape(&self, width: f32, limited: bool) -> Result<Arc<Galley>, LayoutError> {
        self.paragraph.validate()?;
        let mut job = self.paragraph.job(width, limited);
        Ok(self.context.fonts_mut(|fonts| {
            let available = fonts.families();
            for section in &mut job.sections {
                section.format.font_id.family =
                    FontWeight::resolve_family(&section.format.font_id.family, &available);
            }
            fonts.layout_job(job)
        }))
    }
    fn measure(&self, c: BoxConstraints) -> Result<(Arc<Galley>, Vec2), LayoutError> {
        let width = if self.paragraph.soft_wrap || self.paragraph.overflow == TextOverflow::Ellipsis
        {
            c.max().x
        } else {
            f32::INFINITY
        };
        let galley = self.shape(width, true)?;
        let size = c.constrain(galley.size())?;
        Ok((galley, size))
    }
    fn baseline(galley: &Galley, kind: TextBaseline) -> Option<f32> {
        if kind != TextBaseline::Alphabetic {
            return None;
        }
        galley
            .rows
            .first()
            .and_then(|row| row.glyphs.first().map(|g| row.pos.y + g.pos.y))
    }
    fn minimum_width(&self) -> Result<f32, LayoutError> {
        self.paragraph.validate()?;
        // Unicode line-break segments, retaining formatting across byte ranges.
        // Emergency character wrapping in egui does not reduce this contribution.
        let full = self.paragraph.job(f32::INFINITY, false);
        let mut start = 0;
        let mut width = 0.0_f32;
        for (end, _) in unicode_linebreak::linebreaks(&full.text) {
            let segment = &full.text[start..end];
            let trimmed = segment
                .trim_end_matches(|c: char| c.is_whitespace() && c != '\u{a0}' && c != '\u{202f}');
            let end_trimmed = start + trimmed.len();
            let mut job = LayoutJob::default();
            for section in &full.sections {
                let a = start.max(section.byte_range.start);
                let b = end_trimmed.min(section.byte_range.end);
                if a < b {
                    let leading_space = if a == section.byte_range.start {
                        section.leading_space
                    } else {
                        0.0
                    };
                    job.append(&full.text[a..b], leading_space, section.format.clone());
                }
            }
            width = width.max(self.context.fonts_mut(|fonts| {
                let available = fonts.families();
                for section in &mut job.sections {
                    section.format.font_id.family =
                        FontWeight::resolve_family(&section.format.font_id.family, &available);
                }
                fonts.layout_job(job).size().x
            }));
            start = end;
        }
        Ok(width)
    }
}
impl RenderBox for ParagraphRender {
    fn layout_revision(&self) -> u64 {
        // egui exposes no font-collection generation. Refresh once per pass so
        // set_fonts, density and zoom cannot leave retained ancestors stale.
        self.context.cumulative_pass_nr()
    }
    fn dry_layout(&self, c: BoxConstraints, _: &BoxQueries<'_>) -> Result<Vec2, LayoutError> {
        Ok(self.measure(c)?.1)
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
        _: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Ok(Self::baseline(&self.measure(c)?.0, kind))
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        _: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        match d {
            IntrinsicDimension::MinWidth => self.minimum_width(),
            IntrinsicDimension::MaxWidth => Ok(self.shape(f32::INFINITY, false)?.size().x),
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => {
                let width = if self.paragraph.soft_wrap
                    || self.paragraph.overflow == TextOverflow::Ellipsis
                {
                    extent
                } else {
                    f32::INFINITY
                };
                Ok(self.shape(width, true)?.size().y)
            }
        }
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        if !children.is_empty() {
            return Err(LayoutError::InvalidConfiguration(
                "paragraph cannot have render children",
            ));
        }
        let (galley, size) = self.measure(c)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: Self::baseline(&galley, TextBaseline::Alphabetic),
            ideographic: None,
        };
        geometry.clip = if self.paragraph.overflow == TextOverflow::Visible {
            Clip::None
        } else {
            Clip::HardEdge
        };
        self.painted = Some(galley);
        Ok(geometry)
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        if let Some(galley) = &self.painted {
            let origin = ui.max_rect().min
                + egui::vec2(
                    self.paragraph.align.to_factor() * ui.max_rect().width(),
                    0.0,
                );
            ui.painter()
                .galley(origin, galley.clone(), Color32::PLACEHOLDER);
        }
    }
}
