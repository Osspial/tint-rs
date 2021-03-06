// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Utilities for specifying the layout of widgets.
pub use derin_common_types::layout::{Align, Align2, GridSize, Margins, SizeBounds, TrRange, TrackHints, WidgetPos, WidgetSpan};
use crate::core::widget::WidgetIdent;

/// Places widgets in a resizable grid-based layout.
pub trait GridLayout: 'static {
    fn positions(&self, widget_ident: WidgetIdent, widget_index: usize, num_widgets: usize) -> Option<WidgetPos>;
    fn grid_size(&self, num_widgets: usize) -> GridSize;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutHorizontal {
    pub widget_margins: Margins<i32>,
    pub widget_place: Align2
}

impl LayoutHorizontal {
    #[inline(always)]
    pub fn new(widget_margins: Margins<i32>, widget_place: Align2) -> LayoutHorizontal {
        LayoutHorizontal{ widget_margins, widget_place }
    }
}

impl GridLayout for LayoutHorizontal {
    fn positions(&self, _: WidgetIdent, widget_index: usize, num_widgets: usize) -> Option<WidgetPos> {
        match widget_index >= num_widgets {
            true => None,
            false => Some(WidgetPos {
                widget_span: WidgetSpan::new(widget_index as u32, 0),
                margins: self.widget_margins,
                place_in_cell: self.widget_place,
                ..WidgetPos::default()
            })
        }
    }

    #[inline]
    fn grid_size(&self, num_widgets: usize) -> GridSize {
        GridSize::new(num_widgets as u32, 1)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutVertical {
    pub widget_margins: Margins<i32>,
    pub widget_place: Align2
}

impl LayoutVertical {
    #[inline(always)]
    pub fn new(widget_margins: Margins<i32>, widget_place: Align2) -> LayoutVertical {
        LayoutVertical{ widget_margins, widget_place }
    }
}

impl GridLayout for LayoutVertical {
    fn positions(&self, _: WidgetIdent, widget_index: usize, num_widgets: usize) -> Option<WidgetPos> {
        match widget_index >= num_widgets {
            true => None,
            false => Some(WidgetPos {
                widget_span: WidgetSpan::new(0, widget_index as u32),
                margins: self.widget_margins,
                place_in_cell: self.widget_place,
                ..WidgetPos::default()
            })
        }
    }

    #[inline]
    fn grid_size(&self, num_widgets: usize) -> GridSize {
        GridSize::new(1, num_widgets as u32)
    }
}
