use super::{ListView, ScrollError, ScrollPosition};
use crate::{
    element::{BuildContext, FlexItem, FlexStyle, Widget},
    gestures::GestureBinding,
    layout::{Axis, CrossAxisAlignment, Insets},
};
use std::rc::Rc;

type ItemBuilder = dyn Fn(&BuildContext, usize) -> Widget;

/// Fixed-extent lazy grid. Each visible row or column is realized by a
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
    max_realized_items: usize,
    axis: Axis,
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
            max_realized_items: 4096,
            axis: Axis::Vertical,
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

    /// Bound the total number of cells realized across visible and cached
    /// groups. The default is 4096 cells.
    pub fn max_realized_items(mut self, limit: usize) -> Result<Self, ScrollError> {
        if limit == 0 {
            return Err(ScrollError::InvalidRealizedItemLimit);
        }
        self.max_realized_items = limit;
        Ok(self)
    }

    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Set the scrolling direction. The cross-axis count is the number of
    /// columns for vertical scrolling and rows for horizontal scrolling.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn into_widget(self) -> Result<Widget, ScrollError> {
        let group_extent = self.item_extent + self.main_axis_spacing;
        if !group_extent.is_finite() || group_extent <= 0.0 {
            return Err(ScrollError::InvalidGridConfiguration);
        }
        if self.item_count > 0 && self.cross_axis_count > self.max_realized_items {
            return Err(ScrollError::InvalidRealizedItemLimit);
        }
        let group_count = self.item_count.div_ceil(self.cross_axis_count);
        let cells_per_group = self.cross_axis_count;
        let item_count = self.item_count;
        let main_spacing = self.main_axis_spacing;
        let cross_spacing = self.cross_axis_spacing;
        let max_realized_groups = (self.max_realized_items / cells_per_group).max(1);
        let axis = self.axis;
        let cross_axis = match axis {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        };
        let builder = self.builder;
        ListView::builder(
            &self.position,
            &self.binding,
            group_count,
            group_extent,
            move |context, group| {
                let mut cells = Vec::with_capacity(cells_per_group);
                for cross_index in 0..cells_per_group {
                    let index = group * cells_per_group + cross_index;
                    let child = if index < item_count {
                        builder(context, index)
                    } else {
                        Widget::empty()
                    };
                    cells.push(FlexItem::expanded(child, 1));
                }
                let mut style = FlexStyle::new(cross_axis);
                style.spacing = cross_spacing;
                style.cross_alignment = CrossAxisAlignment::Stretch;
                let group = Widget::flex(style, cells);
                Widget::padding(
                    Insets {
                        right: if axis == Axis::Horizontal {
                            main_spacing
                        } else {
                            0.0
                        },
                        bottom: if axis == Axis::Vertical {
                            main_spacing
                        } else {
                            0.0
                        },
                        ..Default::default()
                    },
                    group,
                )
            },
        )?
        .cache_extent(self.cache_extent)?
        .max_realized_items(max_realized_groups)?
        .axis(axis)
        .reverse(self.reverse)
        .into_widget()
    }
}
