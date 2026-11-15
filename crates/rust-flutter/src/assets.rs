//! Embedded bundles and bounded, density-aware image decoding/texture caching.
use egui::{ColorImage, Context, TextureHandle, TextureOptions, Vec2};
pub use rust_flutter_assets::*;
use std::{collections::VecDeque, fmt, io::Cursor};

#[derive(Debug)]
pub enum ImageAssetError {
    Asset(AssetError),
    Decode(image::ImageError),
    Io(std::io::Error),
    TooLarge {
        width: u32,
        height: u32,
        max_bytes: usize,
    },
}
impl fmt::Display for ImageAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asset(e) => e.fmt(f),
            Self::Decode(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
            Self::TooLarge {
                width,
                height,
                max_bytes,
            } => write!(
                f,
                "image {width}x{height} exceeds texture dimensions or {max_bytes}-byte RGBA budget"
            ),
        }
    }
}
impl std::error::Error for ImageAssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Asset(e) => Some(e),
            Self::Decode(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
impl From<AssetError> for ImageAssetError {
    fn from(e: AssetError) -> Self {
        Self::Asset(e)
    }
}
impl From<image::ImageError> for ImageAssetError {
    fn from(e: image::ImageError) -> Self {
        Self::Decode(e)
    }
}
impl From<std::io::Error> for ImageAssetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Clone)]
pub struct AssetTexture {
    pub texture: TextureHandle,
    pub scale: f32,
}
impl AssetTexture {
    /// Display at this size to keep the same layout across density variants.
    pub fn logical_size(&self) -> Vec2 {
        self.texture.size_vec2() / self.scale
    }
    pub fn show(&self, ui: &mut egui::Ui) -> egui::Response {
        ui.image((self.texture.id(), self.logical_size()))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ImageCacheLimits {
    /// Retained RGBA texture bytes and maximum RGBA bytes of one decoded image.
    pub max_bytes: usize,
    pub max_entries: usize,
}
impl Default for ImageCacheLimits {
    fn default() -> Self {
        Self {
            max_bytes: 32 * 1024 * 1024,
            max_entries: 128,
        }
    }
}
struct Cached {
    path: String,
    scale: f32,
    bytes: usize,
    image: AssetTexture,
}
/// LRU cache bound to one immutable bundle. Repeated frames reuse decoded textures.
/// Changing egui contexts clears the cache, avoiding cross-host texture reuse.
/// Cloned `AssetTexture`s can retain textures beyond the cache's memory budget.
/// Decode is synchronous; preload large assets outside the frame-building path.
pub struct AssetImages<B: AssetBundle> {
    bundle: B,
    limits: ImageCacheLimits,
    context: Option<Context>,
    entries: VecDeque<Cached>,
    bytes: usize,
}
impl<B: AssetBundle> AssetImages<B> {
    pub fn new(bundle: B) -> Self {
        Self::with_limits(bundle, ImageCacheLimits::default())
    }
    pub fn with_limits(bundle: B, limits: ImageCacheLimits) -> Self {
        Self {
            bundle,
            limits,
            context: None,
            entries: VecDeque::new(),
            bytes: 0,
        }
    }
    pub fn bundle(&self) -> &B {
        &self.bundle
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    pub fn cached_entries(&self) -> usize {
        self.entries.len()
    }
    pub fn cached_bytes(&self) -> usize {
        self.bytes
    }
    pub fn load(&mut self, ctx: &Context, key: &str) -> Result<AssetTexture, ImageAssetError> {
        self.load_at_density(ctx, key, ctx.pixels_per_point())
    }
    pub fn load_at_density(
        &mut self,
        ctx: &Context,
        key: &str,
        density: f32,
    ) -> Result<AssetTexture, ImageAssetError> {
        if self.context.as_ref() != Some(ctx) {
            self.clear();
            self.context = Some(ctx.clone());
        }
        let asset = self.bundle.resolve(key, density)?;
        if !asset.scale.is_finite() || asset.scale <= 0.0 {
            return Err(AssetError::InvalidDensity(asset.scale).into());
        }
        if let Some(position) = self
            .entries
            .iter()
            .position(|e| e.path == asset.path && e.scale == asset.scale)
        {
            let cached = self.entries.remove(position).unwrap();
            let result = cached.image.clone();
            self.entries.push_front(cached);
            return Ok(result);
        }
        let max_side = ctx.input(|i| i.max_texture_side).min(u32::MAX as usize) as u32;
        let reader = || image::ImageReader::new(Cursor::new(asset.bytes)).with_guessed_format();
        let (width, height) = reader()?.into_dimensions()?;
        let size = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|p| p.checked_mul(4));
        if width == 0
            || height == 0
            || width > max_side
            || height > max_side
            || size.is_none_or(|n| n > self.limits.max_bytes as u64)
        {
            return Err(ImageAssetError::TooLarge {
                width,
                height,
                max_bytes: self.limits.max_bytes,
            });
        }
        let bytes = size.unwrap() as usize;
        let mut decoder = reader()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(max_side);
        limits.max_image_height = Some(max_side);
        limits.max_alloc = Some(self.limits.max_bytes as u64);
        decoder.limits(limits);
        let rgba = decoder.decode()?.into_rgba8();
        let pixels =
            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba.as_raw());
        // Evict before allocating the new retained texture. Decoder and conversion
        // scratch allocations are separate from the retained RGBA texture budget.
        while self.entries.len() >= self.limits.max_entries.max(1)
            || self.bytes > self.limits.max_bytes - bytes
        {
            if let Some(old) = self.entries.pop_back() {
                self.bytes -= old.bytes;
            } else {
                break;
            }
        }
        let result = AssetTexture {
            texture: ctx.load_texture(asset.path, pixels, TextureOptions::LINEAR),
            scale: asset.scale,
        };
        if self.limits.max_entries > 0 {
            self.entries.push_front(Cached {
                path: asset.path.into(),
                scale: asset.scale,
                bytes,
                image: result.clone(),
            });
            self.bytes += bytes;
        }
        Ok(result)
    }
}
