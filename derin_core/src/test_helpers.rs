use crate::{
    LoopFlow,
    event::{EventOps, InputState, WidgetEvent},
    popup::ChildPopupsMut,
    render::{FrameRectStack, RenderFrame, Theme},
    tree::{
        *,
        dynamic::ParentDyn,
    },
};
use cgmath_geometry::{
    D2,
    rect::BoundBox,
};
use derin_common_types::layout::SizeBounds;
use indexmap::IndexMap;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{self, Sender}
};

pub(crate) struct TestWidget {
    pub widget_tag: WidgetTag,
    pub rect: BoundBox<D2, i32>,
    pub size_bounds: SizeBounds,
    pub event_list: Sender<TestEvent>,
    pub children: Option<IndexMap<WidgetIdent, TestWidget>>,
}

#[derive(Clone)]
pub(crate) struct EventList {
    events: Rc<RefCell<std::vec::IntoIter<TestEvent>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TestEvent {
    pub widget: WidgetID,
    pub event: WidgetEvent,
    pub source_child: Vec<WidgetIdent>,
}

#[derive(Default)]
pub(crate) struct TestRenderFrame {}
#[derive(Default)]
pub(crate) struct TestTheme {}
#[derive(Default)]
pub(crate) struct TestAction {}

impl Theme for TestTheme {
    type Key = ();
    type ThemeValue = ();

    fn widget_theme(&self, key: &()) {}
}

impl EventList {
    pub fn new(events: Vec<TestEvent>) -> EventList {
        EventList {
            events: Rc::new(RefCell::new(events.into_iter()))
        }
    }

    fn next(&self) -> Option<TestEvent> {
        self.events.borrow_mut().next()
    }
}

impl RenderFrame for TestRenderFrame {
    type Theme = TestTheme;
    type Primitive = ();

    fn upload_primitives<I>(
        &mut self,
        _theme: &TestTheme,
        _transform: BoundBox<D2, i32>,
        _clip: BoundBox<D2, i32>,
        _prim_iter: I
    )
        where I: Iterator<Item=()>
    {}
}

impl Widget<TestAction, TestRenderFrame> for TestWidget {
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    fn rect(&self) -> BoundBox<D2, i32> {
        self.rect
    }

    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        &mut self.rect
    }

    fn render(&mut self, _frame: &mut FrameRectStack<TestRenderFrame>) {}
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: InputState,
        popups: Option<ChildPopupsMut<TestAction, TestRenderFrame>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<TestAction, TestRenderFrame> {
        let ref_event = self.event_list.next();
        println!("ref event: {:#?}", ref_event);

        let real_event = TestEvent {
            widget: self.widget_tag.widget_id,
            event,
            source_child: source_child.to_vec()
        };
        assert_eq!(ref_event, Some(real_event), "ref event mismatched w/ real event: {:#?}", real_event);

        EventOps::default()
    }

    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    fn as_parent(&self) -> Option<&ParentDyn<TestAction, TestRenderFrame>> {
        if self.children.is_some() {
            Some(self as _)
        } else {
            None
        }
    }

    fn as_parent_mut(&mut self) -> Option<&mut ParentDyn<TestAction, TestRenderFrame>> {
        if self.children.is_some() {
            Some(self as _)
        } else {
            None
        }
    }
}

impl Parent<TestAction, TestRenderFrame> for TestWidget {
    fn num_children(&self) -> usize {
        self.children.as_ref().unwrap().len()
    }

    fn child(&self, ident: WidgetIdent) -> Option<WidgetSummary<&Widget<TestAction, TestRenderFrame>>> {
        self.children.as_ref().unwrap().get_full(&ident)
            .map(|(index, _, widget)| WidgetSummary { ident, index, widget: widget as _ })
    }
    fn child_mut(&mut self, ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<TestAction, TestRenderFrame>>> {
        self.children.as_mut().unwrap().get_full_mut(&ident)
            .map(|(index, _, widget)| WidgetSummary { ident, index, widget: widget as _ })
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<TestAction, TestRenderFrame>>> {
        self.children.as_ref().unwrap().get_index(index)
            .map(|(ident, widget)| WidgetSummary { ident: ident.clone(), index, widget: widget as _ })
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<TestAction, TestRenderFrame>>> {
        self.children.as_mut().unwrap().get_index_mut(index)
            .map(|(ident, widget)| WidgetSummary { ident: ident.clone(), index, widget: widget as _ })
    }

    fn children<'a, G, R>(&'a self, mut for_each: G) -> Option<R>
        where TestAction: 'a,
              G: FnMut(WidgetSummary<&'a Widget<TestAction, TestRenderFrame>>) -> LoopFlow<R>
    {
        for (index, (ident, widget)) in self.children.as_ref().unwrap().iter().enumerate() {
            let flow = for_each(WidgetSummary { ident: ident.clone(), index, widget: widget as _ });
            if let LoopFlow::Break(r) = flow {
                return Some(r);
            }
        }

        None
    }
    fn children_mut<'a, G, R>(&'a mut self, mut for_each: G) -> Option<R>
        where TestAction: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<TestAction, TestRenderFrame>>) -> LoopFlow<R>
    {
        for (index, (ident, widget)) in self.children.as_mut().unwrap().iter_mut().enumerate() {
            let flow = for_each(WidgetSummary { ident: ident.clone(), index, widget: widget as _ });
            if let LoopFlow::Break(r) = flow {
                return Some(r);
            }
        }

        None
    }

    fn update_child_layout(&mut self) {}
}

macro_rules! extract_widget_tree_idents {
    ($($widget_ident:ident {
        rect: ($x:expr, $y:expr, $w:expr, $h:expr)
        $(;$($children:tt)*)*
    }),*) => {$(
        let $widget_ident = WidgetID::new();
        println!("widget {} = {:?}", stringify!($widget_ident), $widget_ident);

        extract_widget_tree_idents!{$($($children)*)*}
    )*};
}

macro_rules! test_widget_tree {
    (
        let $sender_ident:ident = $sender_expr:expr;
        let $root_pat:pat = $root:ident {
            rect: ($x:expr, $y:expr, $w:expr, $h:expr)
            $(;$($rest:tt)*)*
        };
    ) => {
        extract_widget_tree_idents!{
            $root {
                rect: ($x, $y, $w, $h)
                $(;$($rest)*)*
            }
        }
        let $sender_ident = $sender_expr;
        let $root_pat = {
            #[allow(unused_mut)]
            {
                use std::sync::Arc;
                let mut children = indexmap::IndexMap::new();
                test_widget_tree!(
                    @insert
                    $sender_ident,
                    children,
                    $($($rest)*)*
                );

                let mut widget_tag = crate::tree::WidgetTag::new();
                widget_tag.widget_id = $root;

                let root = crate::test_helpers::TestWidget {
                    widget_tag,
                    rect: cgmath_geometry::rect::BoundBox::new2($x, $y, $w, $h),
                    size_bounds: derin_common_types::layout::SizeBounds::default(),
                    event_sender: $sender_ident.clone(),
                    children: match children.len() {
                        0 => None,
                        _ => Some(children)
                    }
                };
                root
            }
        };
    };
    (
        @insert $sender_ident:expr, $widget_map:ident,
        $($child:ident {
            rect: ($x:expr, $y:expr, $w:expr, $h:expr)
            $(;$($children:tt)*)*
        }),*
    ) => {$({
        let mut children = indexmap::IndexMap::new();
        test_widget_tree!(
            @insert
            $sender_ident,
            children,
            $($($children)*)*
        );

        let mut widget_tag = WidgetTag::new();
        widget_tag.widget_id = $child;

        let widget = crate::test_helpers::TestWidget {
            widget_tag,
            rect: cgmath_geometry::rect::BoundBox::new2($x, $y, $w, $h),
            size_bounds: derin_common_types::layout::SizeBounds::default(),
            event_sender: $sender_ident.clone(),
            children: match children.len() {
                0 => None,
                _ => Some(children)
            }
        };

        $widget_map.insert(WidgetIdent::Str(Arc::from(stringify!($child))), widget);
    })*};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_child_widget(
        parent: &dyn ParentDyn<TestAction, TestRenderFrame>,
        index: usize,
        ident: WidgetIdent,
        id: WidgetID,
        rect: BoundBox<D2, i32>,
    ) -> &dyn Widget<TestAction, TestRenderFrame> {
        let summary_by_ident = parent.child(ident.clone()).expect(&format!("Could not find child by ident: {} {:?}", index, ident));
        let summary_by_index = parent.child_by_index(index).expect(&format!("Could not find child by index: {} {:?}", index, ident));

        assert_eq!(summary_by_ident.widget.widget_tag().widget_id, summary_by_index.widget.widget_tag().widget_id);
        assert_eq!(summary_by_ident.widget.widget_tag().widget_id, id);

        assert_eq!(summary_by_ident.ident, ident);
        assert_eq!(summary_by_index.ident, ident);

        assert_eq!(summary_by_ident.index, index);
        assert_eq!(summary_by_index.index, index);

        assert_eq!(summary_by_ident.widget.rect(), rect);
        assert_eq!(summary_by_index.widget.rect(), rect);

        summary_by_ident.widget
    }

    #[test]
    fn widget_tree_macro() {
        let (tx, rx) = mpsc::channel();
        test_widget_tree!{
            let sender = tx;
            let tree = root {
                rect: (0, 0, 500, 500);
                left {
                    rect: (10, 10, 240, 490);
                    tl {rect: (10, 10, 220, 230)},
                    bl {rect: (10, 250, 220, 470)}
                },
                right {rect: (260, 10, 490, 490)}
            };
        }

        assert_eq!(tree.widget_tag().widget_id, root);
        assert_eq!(tree.rect(), BoundBox::new2(0, 0, 500, 500));

        let root_as_parent = tree.as_parent().unwrap();
        let left_widget = check_child_widget(root_as_parent, 0, WidgetIdent::new_str("left"), left, BoundBox::new2(10, 10, 240, 490));
        let right_widget = check_child_widget(root_as_parent, 1, WidgetIdent::new_str("right"), right, BoundBox::new2(260, 10, 490, 490));

        assert!(right_widget.as_parent().is_none());

        let left_widget_as_parent = left_widget.as_parent().unwrap();
        check_child_widget(left_widget_as_parent, 0, WidgetIdent::new_str("tl"), tl, BoundBox::new2(10, 10, 220, 230));
        check_child_widget(left_widget_as_parent, 1, WidgetIdent::new_str("bl"), bl, BoundBox::new2(10, 250, 220, 470));
    }
}