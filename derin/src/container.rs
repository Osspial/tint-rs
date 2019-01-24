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
use std::marker::PhantomData;

use crate::core::LoopFlow;
use crate::core::render::RenderFrame;
use crate::core::widget::{WidgetIdent, WidgetSummary, Widget};

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
pub trait WidgetContainer<F: RenderFrame>: 'static {
    type Widget: ?Sized + Widget<F>;

    /// Get the number of children stored within the container.
    fn num_children(&self) -> usize;

    /// Perform internal, immutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn children<'a, G>(&'a self, for_each_child: G)
        where G: FnMut(WidgetSummary<&'a Self::Widget>) -> LoopFlow,
              F: 'a;

    /// Perform internal, mutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn children_mut<'a, G>(&'a mut self, for_each_child: G)
        where G: FnMut(WidgetSummary<&'a mut Self::Widget>) -> LoopFlow,
              F: 'a;

    /// Get the child with the specified name.
    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Self::Widget>> {
        let mut summary_opt = None;
        self.children(|summary| {
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
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Self::Widget>> {
        let mut summary_opt = None;
        self.children_mut(|summary| {
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
    fn child_by_index(&self, mut index: usize) -> Option<WidgetSummary<&Self::Widget>> {
        let mut summary_opt = None;
        self.children(|summary| {
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
    fn child_by_index_mut(&mut self, mut index: usize) -> Option<WidgetSummary<&mut Self::Widget>> {
        let mut summary_opt = None;
        self.children_mut(|summary| {
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
}

/// A container that contains a single widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SingleContainer<F: RenderFrame, N: Widget<F>> {
    /// A widget.
    pub widget: N,
    _marker: PhantomData<(F)>
}

impl<F: RenderFrame, N: Widget<F>> SingleContainer<F, N> {
    /// Creates a new container containing the given widget.
    #[inline(always)]
    pub fn new(widget: N) -> SingleContainer<F, N> {
        SingleContainer{ widget, _marker: PhantomData }
    }
}

impl<F, N> WidgetContainer<F> for SingleContainer<F, N>
    where F: RenderFrame,
          N: 'static + Widget<F>
{
    type Widget = N;

    #[inline(always)]
    fn num_children(&self) -> usize {1}

    fn children<'a, G>(&'a self, mut for_each_child: G)
        where G: FnMut(WidgetSummary<&'a N>) -> LoopFlow,
              F: 'a
    {
        let _ = for_each_child(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget));
    }

    fn children_mut<'a, G>(&'a mut self, mut for_each_child: G)
        where G: FnMut(WidgetSummary<&'a mut N>) -> LoopFlow,
              F: 'a
    {
        let _ = for_each_child(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget));
    }
}

impl<F, W> WidgetContainer<F> for Vec<W>
    where F: RenderFrame,
          W: 'static + Widget<F>
{
    type Widget = W;

    #[inline(always)]
    fn num_children(&self) -> usize {
        self.len()
    }

    fn children<'a, G>(&'a self, mut for_each_child: G)
        where G: FnMut(WidgetSummary<&'a W>) -> LoopFlow,
              F: 'a
    {
        for (index, widget) in self.iter().enumerate() {
            match for_each_child(WidgetSummary::new(WidgetIdent::Num(index as u32), index, widget)) {
                LoopFlow::Continue => (),
                LoopFlow::Break => return
            }
        }
    }

    fn children_mut<'a, G>(&'a mut self, mut for_each_child: G)
        where G: FnMut(WidgetSummary<&'a mut W>) -> LoopFlow,
              F: 'a
    {
        for (index, widget) in self.iter_mut().enumerate() {
            match for_each_child(WidgetSummary::new_mut(WidgetIdent::Num(index as u32), index, widget)) {
                LoopFlow::Continue => (),
                LoopFlow::Break => return
            }
        }
    }
}
