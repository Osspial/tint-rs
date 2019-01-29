// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Types used to specify children of container widgets.
//!
//! This module's primary functionality is in the `WidgetContainer` trait, and an implementation
//! which contains a single widget is provided with the `SingleContainer` struct.

use crate::core::LoopFlow;
use crate::core::render::RenderFrame;
use crate::core::widget::{WidgetIdent, WidgetInfo, WidgetInfoMut, WidgetSubtype, Widget};

/// Designates a struct that contains other widgets.
///
/// This is used in conjunction with container widgets, such as [`Group`]. This usually shouldn't be
/// directly implemented; you're encouraged to derive it with the macro included in `derin_macros`.
/// Using this macro properly requires a few extra annotations within the type:
/// * `#[derin(action = "$action_type")]` is placed on the struct itself, and is used to set the
///   `Action` type.
/// * `#[derin(collection = "$type_in_collection")]` is placed on fields within the struct which aren't
///   themselves widgets, but are instead collections of widgets, such as `Vec`.
///
/// # Example
/// ```ignore
/// pub struct SimpleAction;
///
/// #[derive(WidgetContainer)]
/// #[derin(action = "SimpleAction")]
/// struct Container {
///     label: Label,
///     edit_box: EditBox,
///     #[derin(collection = "Button<Option<GalleryEvent>>")]
///     buttons: Vec<Button<Option<GalleryEvent>>>
/// }
/// ```
pub trait WidgetContainer<S: ?Sized>: 'static {
    /// Get the number of children stored within the container.
    fn num_children(&self) -> usize;

    /// Perform internal, immutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn framed_children<'a, F, G>(&'a self, for_each_child: G)
        where G: FnMut(WidgetInfo<'a, F, S>) -> LoopFlow,
              F: RenderFrame;

    /// Perform internal, mutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn framed_children_mut<'a, F, G>(&'a mut self, for_each_child: G)
        where G: FnMut(WidgetInfoMut<'a, F, S>) -> LoopFlow,
              F: RenderFrame;

    /// Get the child with the specified name.
    fn framed_child<F>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, F, S>>
        where F: RenderFrame
    {
        let mut summary_opt = None;
        self.framed_children(|summary| {
            if summary.ident == widget_ident {
                summary_opt = Some(summary);
                LoopFlow::Break
            } else {
                LoopFlow::Continue
            }
        });
        summary_opt
    }

    /// Mutably get the child with the specified name.
    fn framed_child_mut<F>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, F, S>>
        where F: RenderFrame
    {
        let mut summary_opt = None;
        self.framed_children_mut(|summary| {
            if summary.ident == widget_ident {
                summary_opt = Some(summary);
                LoopFlow::Break
            } else {
                LoopFlow::Continue
            }
        });
        summary_opt
    }

    /// Get the child at the specified index.
    ///
    /// The index of a child is generally assumed to correspond with the order in which the children
    /// are defined within the container.
    fn framed_child_by_index<F>(&self, mut index: usize) -> Option<WidgetInfo<'_, F, S>>
        where F: RenderFrame
    {
        let mut summary_opt = None;
        self.framed_children(|summary| {
            if index == 0 {
                summary_opt = Some(summary);
                LoopFlow::Break
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        });
        summary_opt
    }
    /// Mutably get the child at the specified index.
    ///
    /// The index of a child is generally assumed to correspond with the order in which the children
    /// are defined within the container.
    fn framed_child_by_index_mut<F>(&mut self, mut index: usize) -> Option<WidgetInfoMut<'_, F, S>>
        where F: RenderFrame
    {
        let mut summary_opt = None;
        self.framed_children_mut(|summary| {
            if index == 0 {
                summary_opt = Some(summary);
                LoopFlow::Break
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        });
        summary_opt
    }

    fn children<'a, G>(&'a self, for_each_child: G)
        where G: FnMut(WidgetInfo<'a, !, S>) -> LoopFlow
    {
        self.framed_children::<!, G>(for_each_child)
    }
    fn children_mut<'a, G>(&'a mut self, for_each_child: G)
        where G: FnMut(WidgetInfoMut<'a, !, S>) -> LoopFlow
    {
        self.framed_children_mut::<!, G>(for_each_child)
    }
    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, !, S>> {
        self.framed_child::<!>(widget_ident)
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, !, S>> {
        self.framed_child_mut::<!>(widget_ident)
    }
    fn child_by_index(&self, index: usize) -> Option<WidgetInfo<'_, !, S>> {
        self.framed_child_by_index(index)
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetInfoMut<'_, !, S>> {
        self.framed_child_by_index_mut(index)
    }
}

/// A container that contains a single widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SingleContainer<W: Widget> {
    /// A widget.
    pub widget: W,
}

impl<W: Widget> SingleContainer<W> {
    /// Creates a new container containing the given widget.
    #[inline(always)]
    pub fn new(widget: W) -> SingleContainer<W> {
        SingleContainer{ widget }
    }
}

impl<S, W> WidgetContainer<S> for SingleContainer<W>
    where S: WidgetSubtype<W>,
          W: Widget
{
    #[inline(always)]
    fn num_children(&self) -> usize {1}

    fn framed_children<'a, F, G>(&'a self, mut for_each_child: G)
            where G: FnMut(WidgetInfo<'a, F, S>) -> LoopFlow,
                  F: RenderFrame
    {
        let _ = for_each_child(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.widget));
    }

    fn framed_children_mut<'a, F, G>(&'a mut self, mut for_each_child: G)
            where G: FnMut(WidgetInfoMut<'a, F, S>) -> LoopFlow,
                  F: RenderFrame
    {
        let _ = for_each_child(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.widget));
    }
}

impl<S, W> WidgetContainer<S> for Vec<W>
    where S: WidgetSubtype<W>,
          W: Widget
{
    #[inline(always)]
    fn num_children(&self) -> usize {
        self.len()
    }

    fn framed_children<'a, F, G>(&'a self, mut for_each_child: G)
            where G: FnMut(WidgetInfo<'a, F, S>) -> LoopFlow,
                  F: RenderFrame
    {
        for (index, widget) in self.iter().enumerate() {
            match for_each_child(WidgetInfo::new(WidgetIdent::Num(index as u32), index, widget)) {
                LoopFlow::Continue => (),
                LoopFlow::Break => return
            }
        }
    }

    fn framed_children_mut<'a, F, G>(&'a mut self, mut for_each_child: G)
            where G: FnMut(WidgetInfoMut<'a, F, S>) -> LoopFlow,
                  F: RenderFrame
    {
        for (index, widget) in self.iter_mut().enumerate() {
            match for_each_child(WidgetInfoMut::new(WidgetIdent::Num(index as u32), index, widget)) {
                LoopFlow::Continue => (),
                LoopFlow::Break => return
            }
        }
    }
}
