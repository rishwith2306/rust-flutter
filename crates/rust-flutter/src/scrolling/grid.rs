use super::{ListView, ScrollError, ScrollPosition};
use crate::{
    element::{BuildContext, FlexItem, FlexStyle, Widget},
    gestures::GestureBinding,
    layout::{Axis, CrossAxisAlignment, Insets},
};
use std::rc::Rc;

type ItemBuilder = dyn Fn(&BuildContext, usize) -> Widget;

/// Vertical fixed-extent lazy grid. Each visible row is realized by a
/// [`ListView`], so construction remains proportional to the viewport/cache.
#[derive(Clone)]
pub struct GridView {
    position: ScrollPosition,
    binding: GestureBinding,
    item_count: usize,
    cross_axis_count: usize,
    item_extent: f32,
    main_axis_spacing: f32,
    cross_axis_spacing: f32,
    cache_extent: f32,
    reverse: bool,
    builder: Rc<ItemBuilder>,
}

impl GridView {
    /// Creates a vertical grid with a fixed number of equal-width columns and
    /// a fixed logical-point height per cell.
    pub fn builder(
        position: &ScrollPosition,
        binding: &GestureBinding,
        item_count: usize,
        cross_axis_count: usize,
        item_extent: f32,
        builder: impl Fn(&BuildContext, usize) -> Widget + 'static,
    ) -> Result<Self, ScrollError> {
        if position.is_disposed() || binding.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        if cross_axis_count == 0 {
            return Err(ScrollError::InvalidGridConfiguration);
        }
        if !item_extent.is_finite() || item_extent <= 0.0 {
            return Err(ScrollError::InvalidItemExtent);
        }
        Ok(Self {
            position: position.clone(),
            binding: binding.clone(),
            item_count,
            cross_axis_count,
            item_extent,
            main_axis_spacing: 0.0,
            cross_axis_spacing: 0.0,
            cache_extent: 250.0,
            reverse: false,
            builder: Rc::new(builder),
        })
    }

    pub fn main_axis_spacing(mut self, spacing: f32) -> Result<Self, ScrollError> {
        if !spacing.is_finite() || spacing < 0.0 {
            return Err(ScrollError::InvalidGridConfiguration);
        }
        self.main_axis_spacing = spacing;
        Ok(self)
    }

    pub fn cross_axis_spacing(mut self, spacing: f32) -> Result<Self, ScrollError> {
        if !spacing.is_finite() || spacing < 0.0 {
            return Err(ScrollError::InvalidGridConfiguration);
        }
        self.cross_axis_spacing = spacing;
        Ok(self)
    }

    pub fn cache_extent(mut self, extent: f32) -> Result<Self, ScrollError> {
        if !extent.is_finite() || extent < 0.0 {
            return Err(ScrollError::InvalidCacheExtent);
        }
        self.cache_extent = extent;
        Ok(self)
    }

    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    pub fn into_widget(self) -> Result<Widget, ScrollError> {
        let row_extent = self.item_extent + self.main_axis_spacing;
        if !row_extent.is_finite() || row_extent <= 0.0 {
            return Err(ScrollError::InvalidGridConfiguration);
        }
        let row_count = self.item_count.div_ceil(self.cross_axis_count);
        let columns = self.cross_axis_count;
        let item_count = self.item_count;
        let main_spacing = self.main_axis_spacing;
        let cross_spacing = self.cross_axis_spacing;
        let builder = self.builder;
        ListView::builder(
            &self.position,
            &self.binding,
            row_count,
            row_extent,
            move |context, row| {
                let mut cells = Vec::with_capacity(columns);
                for column in 0..columns {
                    let index = row * columns + column;
                    let child = if index < item_count {
                        builder(context, index)
                    } else {
                        Widget::empty()
                    };
                    cells.push(FlexItem::expanded(child, 1));
                }
                let mut style = FlexStyle::new(Axis::Horizontal);
                style.spacing = cross_spacing;
                style.cross_alignment = CrossAxisAlignment::Stretch;
                let row = Widget::flex(style, cells);
                Widget::padding(
                    Insets {
                        bottom: main_spacing,
                        ..Default::default()
                    },
                    row,
                )
            },
        )?
        .cache_extent(self.cache_extent)?
        .reverse(self.reverse)
        .into_widget()
    }
}
