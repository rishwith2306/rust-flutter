//! Embedded binary/text assets and density-aware resolution, without a UI dependency.
use std::{error::Error, fmt, str::Utf8Error};

#[cfg(feature = "build")]
pub mod build;

/// Build-generated variant: physical path, pixel density and immutable file bytes.
pub type EmbeddedVariant = (&'static str, f32, &'static [u8]);
/// Build-generated logical key and its available variants.
pub type EmbeddedEntry = (&'static str, &'static [EmbeddedVariant]);

#[derive(Debug)]
pub enum AssetError {
    NotFound(String),
    InvalidDensity(f32),
    InvalidBundle(String),
    Utf8 { key: String, source: Utf8Error },
}
impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(key) => write!(f, "asset not found: {key}"),
            Self::InvalidDensity(density) => {
                write!(f, "asset density must be finite and positive: {density}")
            }
            Self::InvalidBundle(key) => write!(f, "asset has invalid or empty variants: {key}"),
            Self::Utf8 { key, source } => write!(f, "asset {key} is not UTF-8: {source}"),
        }
    }
}
impl Error for AssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Utf8 { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A selected asset. `scale` maps physical image pixels to logical UI points.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedAsset<'a> {
    pub path: &'a str,
    pub scale: f32,
    pub bytes: &'a [u8],
}

/// An immutable asset source, replaceable by an application or test implementation.
/// Existing paths must keep their bytes and scale for the lifetime of the source.
pub trait AssetBundle {
    fn resolve(&self, key: &str, pixel_ratio: f32) -> Result<ResolvedAsset<'_>, AssetError>;
    fn load(&self, key: &str) -> Result<&[u8], AssetError> {
        Ok(self.resolve(key, 1.0)?.bytes)
    }
    fn load_string(&self, key: &str) -> Result<&str, AssetError> {
        std::str::from_utf8(self.load(key)?).map_err(|source| AssetError::Utf8 {
            key: key.into(),
            source,
        })
    }
}
impl<T: AssetBundle + ?Sized> AssetBundle for &T {
    fn resolve(&self, key: &str, pixel_ratio: f32) -> Result<ResolvedAsset<'_>, AssetError> {
        (**self).resolve(key, pixel_ratio)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EmbeddedBundle {
    entries: &'static [EmbeddedEntry],
}
impl EmbeddedBundle {
    /// Construct from generated tuples, or hand-authored immutable test data.
    pub const fn new(entries: &'static [EmbeddedEntry]) -> Self {
        Self { entries }
    }
    pub fn keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.entries.iter().map(|entry| entry.0)
    }
}
impl AssetBundle for EmbeddedBundle {
    fn resolve(&self, key: &str, pixel_ratio: f32) -> Result<ResolvedAsset<'_>, AssetError> {
        if !pixel_ratio.is_finite() || pixel_ratio <= 0.0 {
            return Err(AssetError::InvalidDensity(pixel_ratio));
        }
        let variants = self
            .entries
            .iter()
            .find(|entry| entry.0 == key)
            .ok_or_else(|| AssetError::NotFound(key.into()))?
            .1;
        let mut lower: Option<&EmbeddedVariant> = None;
        let mut upper: Option<&EmbeddedVariant> = None;
        for variant in variants {
            if !variant.1.is_finite() || variant.1 <= 0.0 {
                return Err(AssetError::InvalidBundle(key.into()));
            }
            if variant.1 <= pixel_ratio && lower.is_none_or(|v| variant.1 > v.1) {
                lower = Some(variant);
            }
            if variant.1 >= pixel_ratio && upper.is_none_or(|v| variant.1 < v.1) {
                upper = Some(variant);
            }
        }
        let chosen = match (lower, upper) {
            (Some(lo), Some(hi))
                if pixel_ratio < 2.0 || pixel_ratio - lo.1 > hi.1 - pixel_ratio =>
            {
                hi
            }
            (Some(lo), _) => lo,
            (_, Some(hi)) => hi,
            _ => return Err(AssetError::InvalidBundle(key.into())),
        };
        Ok(ResolvedAsset {
            path: chosen.0,
            scale: chosen.1,
            bytes: chosen.2,
        })
    }
}
