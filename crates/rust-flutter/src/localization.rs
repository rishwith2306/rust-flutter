//! Fluent translations, locale negotiation, per-message fallback and plural rules.
//!
//! Formatting preserves Fluent's Unicode isolation marks around interpolated text.
//! This module does not supply fonts, text shaping, OS locale discovery or RTL layout.
use crate::element::{BuildContext, DependencyError, Widget};
pub use fluent_bundle::{FluentArgs, FluentValue};
use fluent_bundle::{FluentBundle, FluentResource};
use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use std::rc::Rc;
use std::{error::Error, fmt};
pub use unic_langid::LanguageIdentifier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalizationError {
    InvalidLocale(String),
    InvalidResource(Vec<String>),
    DuplicateLocale(String),
    MissingMessage(String),
    Format {
        message: String,
        errors: Vec<String>,
    },
}
impl fmt::Display for LocalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for LocalizationError {}

/// Bundles owned by one UI thread. The fallback locale must always have a resource.
pub struct Localizations {
    locales: Vec<LanguageIdentifier>,
    bundles: Vec<FluentBundle<FluentResource>>,
    chain: Vec<usize>,
    requested: Vec<LanguageIdentifier>,
}
impl Localizations {
    pub fn new(fallback: &str, source: impl Into<String>) -> Result<Self, LocalizationError> {
        let locale = parse_locale(fallback)?;
        let bundle = parse_bundle(&locale, source.into())?;
        Ok(Self {
            locales: vec![locale],
            bundles: vec![bundle],
            chain: vec![0],
            requested: Vec::new(),
        })
    }
    /// Makes this catalog available to descendant widgets through inherited
    /// lookup. Replacing the catalog in a rebuilt app root notifies dependents.
    pub fn provide(self, child: Widget) -> Widget {
        Widget::inherited_with(self, |_, _| true, child)
    }

    /// Reads the nearest catalog and subscribes the current build to changes.
    pub fn of(context: &BuildContext) -> Result<Option<Rc<Self>>, DependencyError> {
        context.depend_on::<Self>()
    }
    /// Add one complete FTL resource for a locale. Invalid additions leave the catalog unchanged.
    pub fn add_locale(
        &mut self,
        locale: &str,
        source: impl Into<String>,
    ) -> Result<(), LocalizationError> {
        let locale = parse_locale(locale)?;
        if self.locales.contains(&locale) {
            return Err(LocalizationError::DuplicateLocale(locale.to_string()));
        }
        let bundle = parse_bundle(&locale, source.into())?;
        self.locales.push(locale);
        self.bundles.push(bundle);
        self.negotiate();
        Ok(())
    }
    /// Select preferred locales in priority order. Empty or unmatched preferences use the fallback.
    /// Invalid locale identifiers return an error without changing the previous selection.
    pub fn select(&mut self, requested: &[&str]) -> Result<&LanguageIdentifier, LocalizationError> {
        self.requested = requested
            .iter()
            .map(|s| parse_locale(s))
            .collect::<Result<_, _>>()?;
        self.negotiate();
        Ok(self.locale())
    }
    fn negotiate(&mut self) {
        let matches = negotiate_languages(
            &self.requested,
            &self.locales,
            Some(&self.locales[0]),
            NegotiationStrategy::Filtering,
        );
        self.chain = matches
            .into_iter()
            .map(|locale| self.locales.iter().position(|v| v == locale).unwrap())
            .collect();
    }
    pub fn locale(&self) -> &LanguageIdentifier {
        &self.locales[self.chain[0]]
    }
    pub fn supported(&self) -> &[LanguageIdentifier] {
        &self.locales
    }
    pub fn text(&self, id: &str) -> Result<String, LocalizationError> {
        self.format(id, None)
    }
    pub fn format(
        &self,
        id: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> Result<String, LocalizationError> {
        self.resolve(id, None, args)
    }
    pub fn attribute(
        &self,
        id: &str,
        attribute: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> Result<String, LocalizationError> {
        self.resolve(id, Some(attribute), args)
    }
    fn resolve(
        &self,
        id: &str,
        attribute: Option<&str>,
        args: Option<&FluentArgs<'_>>,
    ) -> Result<String, LocalizationError> {
        for &index in &self.chain {
            let bundle = &self.bundles[index];
            let Some(message) = bundle.get_message(id) else {
                continue;
            };
            let pattern = match attribute {
                Some(name) => message.get_attribute(name).map(|a| a.value()),
                None => message.value(),
            };
            let Some(pattern) = pattern else {
                continue;
            };
            let mut errors = Vec::new();
            let value = bundle.format_pattern(pattern, args, &mut errors);
            if !errors.is_empty() {
                return Err(LocalizationError::Format {
                    message: id.into(),
                    errors: errors.into_iter().map(|e| e.to_string()).collect(),
                });
            }
            return Ok(value.into_owned());
        }
        Err(LocalizationError::MissingMessage(match attribute {
            Some(a) => format!("{id}.{a}"),
            None => id.into(),
        }))
    }
}
fn parse_locale(locale: &str) -> Result<LanguageIdentifier, LocalizationError> {
    locale
        .parse()
        .map_err(|_| LocalizationError::InvalidLocale(locale.into()))
}
fn parse_bundle(
    locale: &LanguageIdentifier,
    source: String,
) -> Result<FluentBundle<FluentResource>, LocalizationError> {
    let resource = FluentResource::try_new(source).map_err(|(_, errors)| {
        LocalizationError::InvalidResource(errors.into_iter().map(|e| e.to_string()).collect())
    })?;
    let mut bundle = FluentBundle::new(vec![locale.clone()]);
    bundle.add_resource(resource).map_err(|errors| {
        LocalizationError::InvalidResource(errors.into_iter().map(|e| e.to_string()).collect())
    })?;
    Ok(bundle)
}
