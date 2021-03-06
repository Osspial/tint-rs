// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    event::{EventOps, WidgetEventSourced, InputState},
    widget::{WidgetTag, WidgetRenderable, Widget},
    render::{Renderer, RendererLayout, SubFrame, WidgetTheme},
};
use crate::widgets::Contents;

use cgmath_geometry::{D2, rect::BoundBox};
use derin_common_types::layout::SizeBounds;


/// A simple, non-interactive label.
///
/// Can display text or an image, depending on what's in `contents`.
#[derive(Debug, Clone)]
pub struct Label {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    contents: Contents,
    size_bounds: SizeBounds,
}

#[derive(Debug, Clone, Default)]
pub struct LabelTheme(());

impl WidgetTheme for LabelTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {None}
}

impl Label {
    /// Create a new label with the given contents.
    pub fn new(contents: Contents) -> Label {
        Label {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            contents,
            size_bounds: SizeBounds::default(),
        }
    }

    /// Retrieves the contents of the label.
    pub fn contents(&self) -> &Contents {
        &self.contents
    }

    /// Retrieves the contents of the label, for mutation.
    ///
    /// Calling this function forces the label to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> &mut Contents {
        self.widget_tag
            .request_redraw()
            .request_relayout();

        &mut self.contents
    }
}

impl Widget for Label {
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        &mut self.bounds
    }

    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl<R> WidgetRenderable<R> for Label
    where R: Renderer
{
    type Theme = LabelTheme;
    fn theme(&self) -> LabelTheme {
        LabelTheme(())
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, layout: &mut R::Layout) {
        match self.contents {
            Contents::Text(ref s) => layout.prepare_string(s),
            Contents::Icon(ref i) => layout.prepare_icon(i),
        }

        let result = layout.finish();
        self.size_bounds = result.size_bounds;
    }
}
