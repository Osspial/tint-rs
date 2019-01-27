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

mod dynamic;
pub(crate) use dynamic::WidgetDyn;
pub use crate::{
    message_bus::MessageTarget,
    update_state::UpdateError,
};

use crate::{
    LoopFlow,
    event::{WidgetEventSourced, EventOps, InputState},
    message_bus::{WidgetMessageKey, WidgetMessageFn},
    render::{RenderFrame, RenderFrameClipped},
    timer::{TimerID, Timer},
    update_state::{UpdateStateShared, UpdateStateCell},
};
use derin_common_types::{
    cursor::CursorIcon,
    layout::SizeBounds,
};
use smallvec::SmallVec;
use std::{
    any::{Any, TypeId},
    borrow::{Borrow, BorrowMut},
    cell::{Cell, RefCell},
    fmt,
    ops::Drop,
    rc::Rc,
    sync::Arc,
};
use cgmath_geometry::{
    D2, rect::BoundBox,
    cgmath::Point2,
};
use fnv::FnvHashMap;


pub(crate) const ROOT_IDENT: WidgetIdent = WidgetIdent::Num(0);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetIdent {
    Str(Arc<str>),
    Num(u32),
    StrCollection(Arc<str>, u32),
    NumCollection(u32, u32)
}

pub struct WidgetTag {
    update_state: RefCell<UpdateStateShared>,
    registered_messages: FnvHashMap<WidgetMessageKey, Cell<SmallVec<[WidgetMessageFn; 1]>>>,
    pub(crate) widget_id: WidgetID,
    pub(crate) timers: FnvHashMap<TimerID, Timer>,
}

impl fmt::Debug for WidgetTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_tuple("WidgetTag")
            .field(&self.widget_id)
            .finish()
    }
}

impl Clone for WidgetTag {
    /// This doesn't actually clone the `WidgetTag` - it just creates a new one and returns it. This
    /// function is provided primarily to allow widgets to cleanly derive `Clone`.
    fn clone(&self) -> WidgetTag {
        WidgetTag::new()
    }
}

id!(pub WidgetID);


/// The base widget trait.
///
/// ## Warnings
/// Note that this trait ***should not be implemented for unsized types***. TODO EXPLAIN WHY
pub trait Widget: 'static {
    fn widget_tag(&self) -> &WidgetTag;
    fn widget_id(&self) -> WidgetID {
        self.widget_tag().widget_id
    }

    fn rect(&self) -> BoundBox<D2, i32>;
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32>;
    fn on_widget_event(
        &mut self,
        event: WidgetEventSourced<'_>,
        input_state: InputState,
    ) -> EventOps;

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::default()
    }

    #[doc(hidden)]
    fn dispatch_message(&mut self, message: &Any) {
        let message_key = WidgetMessageKey::from_dyn_message::<Self>(message);

        // We have to pull the `message_fns` list out of the widget tag so that we can pass self
        // mutably into the message functions.
        let mut message_fns = {
            let message_fns_cell = match self.widget_tag().registered_messages.get(&message_key) {
                Some(afc) => afc,
                None => return
            };
            message_fns_cell.replace(SmallVec::new())
        };

        for f in &mut message_fns {
            dynamic::to_any(self, |w| f(w, message));
        }

        let message_fns_cell = match self.widget_tag().registered_messages.get(&message_key) {
            Some(afc) => afc,
            None => return
        };

        // Pull any new message functions into the canonical `message_fns` list.
        let new_message_fns = message_fns_cell.replace(SmallVec::new());
        message_fns.extend(new_message_fns);

        // Put the canonical `message_fns` list back into the cell.
        message_fns_cell.replace(message_fns);
    }
}

pub trait WidgetRender<F: RenderFrame>: Widget {
    fn render(&mut self, frame: &mut RenderFrameClipped<F>);
    fn update_layout(&mut self, _theme: &F::Theme) {}
}

impl<W> Widget for Box<W>
    where W: Widget + ?Sized
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        W::widget_tag(self)
    }
    fn rect(&self) -> BoundBox<D2, i32> {
        W::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        W::rect_mut(self)
    }
    fn on_widget_event(
        &mut self,
        event: WidgetEventSourced<'_>,
        input_state: InputState,
    ) -> EventOps {
        W::on_widget_event(self, event, input_state)
    }

    fn size_bounds(&self) -> SizeBounds {
        W::size_bounds(self)
    }

    fn dispatch_message(&mut self, message: &Any) {
        W::dispatch_message(self, message)
    }
}

pub struct WidgetInfo<'a, F: RenderFrame, S: ?Sized=Widget> {
    pub ident: WidgetIdent,
    pub index: usize,
    pub(crate) widget: &'a WidgetDyn<F>,
    to_secondary: fn(&'_ WidgetDyn<F>) -> &'_ S,
}

pub struct WidgetInfoMut<'a, F: RenderFrame, S: ?Sized=Widget> {
    pub ident: WidgetIdent,
    pub index: usize,
    pub(crate) widget: &'a mut WidgetDyn<F>,
    to_secondary: fn(Reference<'_, WidgetDyn<F>>) -> Reference<'_, S>
}

enum Reference<'a, T: ?Sized> {
    Ref(&'a T),
    Mut(&'a mut T),
}

impl<'a, T: ?Sized> Reference<'a, T> {
    fn as_ref(&self) -> &T {
        match self {
            Reference::Ref(r) => r,
            Reference::Mut(r) => r
        }
    }
}

pub trait Parent: Widget {
    fn num_children(&self) -> usize;

    fn framed_child<F: RenderFrame>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, F>>
        where Self: Sized;
    fn framed_child_mut<F: RenderFrame>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, F>>
        where Self: Sized;

    fn framed_child_by_index<F: RenderFrame>(&self, index: usize) -> Option<WidgetInfo<'_, F>>
        where Self: Sized;
    fn framed_child_by_index_mut<F: RenderFrame>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, F>>
        where Self: Sized;

    fn framed_children<'a, F, G>(&'a self, for_each: G)
        where Self: Sized,
              F: RenderFrame,
              G: FnMut(WidgetInfo<'a, F>) -> LoopFlow;
    fn framed_children_mut<'a, F, G>(&'a mut self, for_each: G)
        where Self: Sized,
              F: RenderFrame,
              G: FnMut(WidgetInfoMut<'a, F>) -> LoopFlow;
}

pub trait WidgetSubtype<W: Widget> {
    fn from_widget(widget: &W) -> &Self;
    fn from_widget_mut(widget: &mut W) -> &mut Self;
}

impl<W, S> WidgetSubtype<W> for S
    where W: Widget + Borrow<S> + BorrowMut<S>
{
    #[inline(always)]
    fn from_widget(widget: &W) -> &S {
        widget.borrow()
    }

    #[inline(always)]
    fn from_widget_mut(widget: &mut W) -> &mut S {
        widget.borrow_mut()
    }
}

impl<W: Widget> WidgetSubtype<W> for dyn Widget {
    fn from_widget(widget: &W) -> &Self {
        widget
    }
    fn from_widget_mut(widget: &mut W) -> &mut Self {
        widget
    }
}

impl<W: WidgetRender<F>, F: RenderFrame> WidgetSubtype<W> for dyn WidgetRender<F> {
    fn from_widget(widget: &W) -> &Self {
        widget
    }
    fn from_widget_mut(widget: &mut W) -> &mut Self {
        widget
    }
}

impl<W: Parent> WidgetSubtype<W> for dyn Parent {
    fn from_widget(widget: &W) -> &Self {
        widget
    }
    fn from_widget_mut(widget: &mut W) -> &mut Self {
        widget
    }
}

impl<'a, F, S> WidgetInfo<'a, F, S>
    where F: RenderFrame
{
    pub fn new<W>(
        ident: WidgetIdent,
        index: usize,
        widget: &'a W
    ) -> WidgetInfo<'a, F, S>
        where W: Widget,
              S: WidgetSubtype<W>
    {
        WidgetInfo {
            ident,
            index,
            widget: WidgetDyn::new(widget),
            to_secondary: |r| {
                if r.get_type_id() == TypeId::of::<W>() {
                    S::from_widget(unsafe{ &*(r as *const WidgetDyn<F> as *const W) })
                } else {
                    panic!("widget type replaced")
                }
            }
        }
    }

    pub fn erase_subtype(self) -> WidgetInfo<'a, F> {
        WidgetInfo {
            ident: self.ident,
            index: self.index,
            widget: self.widget,
            to_secondary: |r| {
                r.to_widget()
            }
        }
    }
}

impl<'a, F, S> WidgetInfoMut<'a, F, S>
    where F: RenderFrame
{
    pub fn new<W>(ident: WidgetIdent, index: usize, widget: &'a mut W) -> WidgetInfoMut<'a, F, S>
        where W: Widget,
              S: WidgetSubtype<W>
    {
        WidgetInfoMut {
            ident,
            index,
            widget: WidgetDyn::new_mut(widget),
            to_secondary: |r| {
                if r.as_ref().get_type_id() == TypeId::of::<W>() {
                    match r {
                        Reference::Ref(r) =>
                            Reference::Ref(S::from_widget(unsafe{ &*(r as *const WidgetDyn<F> as *const W) })),
                        Reference::Mut(r) =>
                            Reference::Mut(S::from_widget_mut(unsafe{ &mut *(r as *mut WidgetDyn<F> as *mut W) }))
                    }
                } else {
                    panic!("widget type replaced")
                }
            }
        }
    }

    pub fn erase_subtype(self) -> WidgetInfoMut<'a, F> {
        WidgetInfoMut {
            ident: self.ident,
            index: self.index,
            widget: self.widget,
            to_secondary: |r| match r {
                Reference::Ref(r) => Reference::Ref(r.to_widget()),
                Reference::Mut(r) => Reference::Mut(r.to_widget_mut()),
            }
        }
    }
}

impl<'a, F, S> Borrow<S> for WidgetInfo<'a, F, S>
    where F: RenderFrame
{
    fn borrow(&self) -> &S {
        (self.to_secondary)(self.widget)
    }
}

impl<'a, F, S> Borrow<S> for WidgetInfoMut<'a, F, S>
    where F: RenderFrame
{
    fn borrow(&self) -> &S {
        match (self.to_secondary)(Reference::Ref(self.widget)) {
            Reference::Ref(r) => r,
            Reference::Mut(_) => unreachable!()
        }
    }
}

impl<'a, F, S> BorrowMut<S> for WidgetInfoMut<'a, F, S>
    where F: RenderFrame
{
    fn borrow_mut(&mut self) -> &mut S {
        match (self.to_secondary)(Reference::Mut(self.widget)) {
            Reference::Mut(r) => r,
            Reference::Ref(_) => unreachable!()
        }
    }
}

impl WidgetIdent {
    pub fn new_str(s: &str) -> WidgetIdent {
        WidgetIdent::Str(Arc::from(s))
    }

    pub fn new_str_collection(s: &str, i: u32) -> WidgetIdent {
        WidgetIdent::StrCollection(Arc::from(s), i)
    }
}

impl WidgetTag {
    #[inline]
    pub fn new() -> WidgetTag {
        WidgetTag {
            update_state: RefCell::new(UpdateStateShared::new()),
            widget_id: WidgetID::new(),
            registered_messages: FnvHashMap::default(),
            timers: FnvHashMap::default(),
        }
    }

    #[inline]
    pub fn widget_id(&self) -> WidgetID {
        self.widget_id
    }

    #[inline]
    pub fn request_redraw(&mut self) -> &mut WidgetTag {
        self.update_state.get_mut().request_redraw(self.widget_id);
        self
    }

    #[inline]
    pub fn request_relayout(&mut self) -> &mut WidgetTag {
        self.update_state.get_mut().request_relayout(self.widget_id);
        self
    }

    pub fn timers(&self) -> &FnvHashMap<TimerID, Timer> {
        &self.timers
    }

    pub fn timers_mut(&mut self) -> &mut FnvHashMap<TimerID, Timer> {
        self.update_state.get_mut().request_update_timers(self.widget_id);
        &mut self.timers
    }

    pub fn register_message<W, A>(&mut self, mut f: impl 'static + FnMut(&mut W, &A))
        where W: 'static,
              A: 'static
    {
        self.update_state.get_mut().request_update_messages(self.widget_id);

        let f: Box<FnMut(&mut Any, &Any)> = Box::new(move |widget_any, message_any| {
            let widget = widget_any.downcast_mut::<W>().expect("Passed bad widget type to message fn");
            let message = message_any.downcast_ref::<A>().expect("Passed bad message type to message fn");
            f(widget, message);
        });

        self.registered_messages.entry(WidgetMessageKey::new::<W, A>())
            .or_insert(Cell::new(SmallVec::new()))
            .get_mut()
            .push(f);
    }

    pub fn message_types(&self) -> impl '_ + Iterator<Item=TypeId> {
        self.registered_messages.keys().map(|k| k.message_type())
    }

    pub fn broadcast_message<A: 'static>(&mut self, message: A) {
        self.update_state.get_mut().send_message(message, None);
    }

    pub fn send_message_to<A: 'static>(&mut self, message: A, target: MessageTarget) {
        self.update_state.get_mut().send_message(message, Some(target));
    }

    pub fn set_cursor_pos(&mut self, cursor_pos: Point2<i32>) -> Result<(), UpdateError> {
        self.update_state.get_mut().request_set_cursor_pos(self.widget_id, cursor_pos)
    }

    pub fn set_cursor_icon(&mut self, cursor_icon: CursorIcon) -> Result<(), UpdateError> {
        self.update_state.get_mut().request_set_cursor_icon(cursor_icon)
    }

    #[inline]
    pub fn has_keyboard_focus(&self) -> bool {
        unimplemented!()
    }

    #[inline]
    pub(crate) fn set_owning_update_state(&self, state: &Rc<UpdateStateCell>) {
        self.update_state.borrow_mut().set_owning_update_state(self.widget_id, state);
    }
}

impl Drop for WidgetTag {
    fn drop(&mut self) {
        self.update_state.get_mut().remove_from_tree(self.widget_id)
    }
}
