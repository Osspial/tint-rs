mod dispatcher;

use crate::{
    WindowEvent, InputState, LoopFlow,
    event::WidgetEvent,
    tree::*,
    timer::TimerList,
    render::RenderFrame,
    widget_stack::{WidgetStack, WidgetStackBase, WidgetPath},
    update_state::{UpdateStateBuffered},
    offset_widget::{OffsetWidgetTrait, OffsetWidgetTraitAs},
    virtual_widget_tree::VirtualWidgetTree
};
use self::dispatcher::{EventDispatcher, EventDestination, DispatchableEvent};
use cgmath_geometry::rect::GeoBox;
use std::{
    rc::Rc,
    iter::{ExactSizeIterator, DoubleEndedIterator}
};

pub(crate) struct EventTranslator<A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    widget_stack_base: WidgetStackBase<A, F>,
    inner: TranslatorInner<A>
}

pub(crate) struct TranslatorActive<'a, A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    widget_stack: WidgetStack<'a, A, F>,
    inner: &'a mut TranslatorInner<A>
}

struct TranslatorInner<A: 'static> {
    actions: Vec<A>,
    timer_list: TimerList,
    event_dispatcher: EventDispatcher,
    virtual_widget_tree: VirtualWidgetTree,
    update_state: Rc<UpdateStateBuffered>,
}

impl<A, F> EventTranslator<A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    pub fn new(root_id: WidgetID) -> EventTranslator<A, F> {
        EventTranslator {
            widget_stack_base: WidgetStackBase::new(),
            inner: TranslatorInner {
                actions: Vec::new(),
                timer_list: TimerList::new(None),
                event_dispatcher: EventDispatcher::new(),
                virtual_widget_tree: VirtualWidgetTree::new(root_id),
                update_state: UpdateStateBuffered::new(),
            },
        }
    }

    pub fn with_translator<'a>(&'a mut self, root: &'a mut Widget<A, F>) -> TranslatorActive<'a, A, F> {
        TranslatorActive {
            widget_stack: self.widget_stack_base.use_stack_dyn(root),
            inner: &mut self.inner
        }
    }

    pub fn drain_actions(&mut self) -> impl '_ + Iterator<Item=A> + ExactSizeIterator + DoubleEndedIterator {
        self.inner.actions.drain(..)
    }
}

impl<'a, A, F> TranslatorActive<'a, A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    pub fn translate_window_event(&mut self, window_event: WindowEvent, input_state: &mut InputState) {
        use self::WindowEvent::*;

        let TranslatorActive {
            ref mut widget_stack,
            ref mut inner
        } = self;
        let TranslatorInner {
            ref mut actions,
            ref mut timer_list,
            ref mut event_dispatcher,
            ref mut virtual_widget_tree,
            ref update_state
        } = inner;

        let mut queue_hover_event = |event| {
            let old_hover_widget_opt = match input_state.mouse_hover_widget {
                Some(w) => widget_stack.move_to_widget_with_tree(w, virtual_widget_tree),
                None => None
            };

            if let Some(old_hover_widget) = old_hover_widget_opt {
                event_dispatcher.queue_event(
                    EventDestination::Widget(old_hover_widget.widget.widget_tag().widget_id),
                    DispatchableEvent::WidgetEvent {
                        bubble_source: None,
                        event: event
                    }
                );
            }
        };

        match window_event {
            MouseMove(new_pos) => {
                let old_pos = input_state.mouse_pos.unwrap_or(new_pos);
                input_state.mouse_pos = Some(new_pos);

                let old_hover_widget_opt = match input_state.mouse_hover_widget {
                    Some(w) => widget_stack.move_to_widget_with_tree(w, virtual_widget_tree),
                    None => None
                };

                if let Some(ohw) = old_hover_widget_opt {
                    let mut ohw = ohw.widget;
                    let ohw_id = ohw.widget_tag().widget_id;
                    event_dispatcher.queue_event(
                        EventDestination::Widget(ohw_id),
                        DispatchableEvent::WidgetEvent {
                            bubble_source: None,
                            event: WidgetEvent::MouseMove {
                                old_pos, new_pos,
                                in_widget: true // This is a tentative value. The real value is calculated in the dispatch function.
                            }
                        }
                    );

                } else if let Some(hover_widget) = widget_stack.move_to_path(Some(ROOT_IDENT)).filter(|w| w.widget.rect().contains(new_pos)) {
                    let hover_id = hover_widget.widget.widget_tag().widget_id;
                    event_dispatcher.queue_event(
                        EventDestination::Widget(hover_id),
                        DispatchableEvent::WidgetEvent {
                            bubble_source: None,
                            event: WidgetEvent::MouseEnter
                        }
                    );
                    event_dispatcher.queue_event(
                        EventDestination::Widget(hover_id),
                        DispatchableEvent::WidgetEvent {
                            bubble_source: None,
                            event: WidgetEvent::MouseMove {
                                old_pos, new_pos,
                                in_widget: true
                            }
                        }
                    );
                }
            },
            MouseEnter => (),
            MouseExit => {
                queue_hover_event(WidgetEvent::MouseExit);
                if input_state.mouse_buttons_down.len() == 0 {
                    input_state.mouse_pos = None;
                }
            }
            MouseDown(mouse_button) => {
                if let Some(mouse_pos) = input_state.mouse_pos {
                    queue_hover_event(WidgetEvent::MouseDown {
                        pos: mouse_pos,
                        in_widget: true,
                        button: mouse_button
                    });
                    input_state.mouse_buttons_down.push_button(mouse_button, mouse_pos);
                }
            },
            MouseUp(mouse_button) => {
                if let Some(mouse_pos) = input_state.mouse_pos {
                    if let Some(mouse_down) = input_state.mouse_buttons_down.contains(mouse_button) {
                        queue_hover_event(WidgetEvent::MouseUp {
                            pos: mouse_pos,
                            down_pos: mouse_down.down_pos,
                            pressed_in_widget: unimplemented!(),
                            in_widget: true,
                            button: mouse_button
                        });
                        input_state.mouse_buttons_down.release_button(mouse_button);
                    }
                }
            },
            MouseScrollLines(dir) => queue_hover_event(WidgetEvent::MouseScrollLines(dir)),
            MouseScrollPx(dir) => queue_hover_event(WidgetEvent::MouseScrollPx(dir)),
            WindowResize(_) => unimplemented!(),
            KeyDown(key) => {
                if crate::find_index(&input_state.keys_down, &key).is_none() {
                    input_state.keys_down.push(key);
                    match input_state.focused_widget {
                        Some(widget) => event_dispatcher.queue_event(
                            EventDestination::Widget(widget),
                            DispatchableEvent::WidgetEvent {
                                bubble_source: None,
                                event: WidgetEvent::KeyDown(key, input_state.modifiers)
                            }
                        ),
                        None => unimplemented!("dispatch to universal fallthrough")
                    }
                }
            },
            KeyUp(key) => {
                if crate::vec_remove_element(&mut input_state.keys_down, &key).is_some() {
                    match input_state.focused_widget {
                        Some(widget) => event_dispatcher.queue_event(
                            EventDestination::Widget(widget),
                            DispatchableEvent::WidgetEvent {
                                bubble_source: None,
                                event: WidgetEvent::KeyUp(key, input_state.modifiers)
                            }
                        ),
                        None => unimplemented!("dispatch to universal fallthrough")
                    }
                }
            },
            Char(c) => {
                match input_state.focused_widget {
                    Some(widget) => event_dispatcher.queue_event(
                        EventDestination::Widget(widget),
                        DispatchableEvent::WidgetEvent {
                            bubble_source: None,
                            event: WidgetEvent::Char(c)
                        }
                    ),
                    None => unimplemented!("dispatch to universal fallthrough")
                }
            },
            Timer => unimplemented!(),
            Redraw => unimplemented!()
        }
    }

    pub fn dispatch_events(&mut self, input_state: &mut InputState) {
        let TranslatorActive {
            ref mut widget_stack,
            ref mut inner
        } = self;
        let TranslatorInner {
            ref mut actions,
            ref mut timer_list,
            ref mut event_dispatcher,
            ref mut virtual_widget_tree,
            ref update_state
        } = inner;

        event_dispatcher.dispatch_events(
            widget_stack,
            virtual_widget_tree,
            |event_dispatcher, WidgetPath{widget, path, widget_id, ..}, event| {
                let perform_event_ops = |ops| {
                    use crate::event::{EventOps, FocusChange};
                    let EventOps {
                        action,
                        focus,
                        bubble,
                        cursor_pos,
                        cursor_icon,
                        popup
                    } = ops;
                    if let Some(action) = action {
                        actions.push(action);
                    }
                    if let Some(focus) = focus {
                        let destination = match focus {
                            FocusChange::Next =>
                        }
                    }
                };

                match event {
                    DispatchableEvent::WidgetEvent{ bubble_source, event } => match event {
                        WidgetEvent::MouseMove{old_pos, new_pos, in_widget} if bubble_source.is_none() => {
                            if widget.rect().contains(new_pos) {
                                let mut enter_child_opt = None;
                                if let Some(widget_as_parent) = widget.as_parent_mut() {
                                    widget_as_parent.children_mut(|child_summary| {
                                        if child_summary.widget.rect().contains(new_pos) {
                                            enter_child_opt = Some(child_summary.widget.widget_tag().widget_id);
                                            LoopFlow::Break(())
                                        } else {
                                            LoopFlow::Continue
                                        }
                                    });
                                }

                                match enter_child_opt {
                                    Some(enter_child) => {
                                        perform_event_ops(widget.on_widget_event(
                                            WidgetEvent::MouseMove{old_pos, new_pos, in_widget: false},
                                            input_state,
                                            None, // TODO: POPUPS
                                            &[]
                                        ));
                                        event_dispatcher.queue_event(
                                            EventDestination::Widget(enter_child),
                                            DispatchableEvent::WidgetEvent {
                                                bubble_source: None,
                                                event: WidgetEvent::MouseEnter
                                            }
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        );
    }
}
