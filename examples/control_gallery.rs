#![feature(never_type)]

extern crate derin;
#[macro_use]
extern crate derin_macros;

use derin::ui::*;
use derin::ui::widgets::*;
use derin::ui::widgets::content::{Orientation, Completion, ProgbarStatus, SliderStatus};
use derin::ui::hints::*;

use derin::native::{Window, WindowConfig};

use std::iter;

enum GalleryEvent {
    AddButton,
    DelButton,
    SliderMoved(u32)
}

struct AddButton;
impl EventActionMap<MouseEvent> for AddButton {
    type Action = GalleryEvent;

    fn on_event(&self, _: MouseEvent) -> Option<GalleryEvent> {
        Some(GalleryEvent::AddButton)
    }
}

struct DelButton;
impl EventActionMap<MouseEvent> for DelButton {
    type Action = GalleryEvent;

    fn on_event(&self, _: MouseEvent) -> Option<GalleryEvent> {
        Some(GalleryEvent::DelButton)
    }
}

struct BasicSlider;
impl EventActionMap<RangeEvent> for BasicSlider {
    type Action = GalleryEvent;

    fn on_event(&self, event: RangeEvent) -> Option<GalleryEvent> {
        if let RangeEvent::Move(moved_to) = event {
            Some(GalleryEvent::SliderMoved(moved_to))
        } else {
            None
        }
    }
}

#[derive(Parent)]
#[derin(child_action = "GalleryEvent")]
struct BasicParent {
    label: TextLabel<&'static str>,
    bar: Progbar,
    slider: Slider<BasicSlider>,
    nested_parent: LabelGroup<&'static str, NestedParent>,
    #[derin(layout)]
    layout: BasicParentLayout
}

impl BasicParent {
    fn new() -> BasicParent {
        BasicParent {
            label: TextLabel::new("A Label"),
            bar: Progbar::new(ProgbarStatus::new(Completion::Frac(0.5), Orientation::Horizontal)),
            slider: Slider::new(BasicSlider, SliderStatus::default()),
            nested_parent: LabelGroup::new("Hello World", NestedParent {
                del_button: TextButton::new(DelButton, "Delete Button"),
                add_button: TextButton::new(AddButton, "Add Button"),
                button_vec: Vec::new(),
                layout: NestedParentLayout
            }),
            layout: BasicParentLayout
        }
    }
}

#[derive(Parent)]
#[derin(child_action = "GalleryEvent")]
struct NestedParent {
    del_button: TextButton<DelButton, &'static str>,
    add_button: TextButton<AddButton, &'static str>,
    #[derin(collection)]
    button_vec: Vec<TextButton<AddButton, &'static str>>,
    #[derin(layout)]
    layout: NestedParentLayout
}

struct BasicParentLayout;
struct NestedParentLayout;

impl<'a> GridLayout<'a> for BasicParentLayout {
    type ColHints = iter::Repeat<TrackHints>;
    type RowHints = iter::Repeat<TrackHints>;

    fn grid_size(&self) -> GridSize {
        GridSize::new(1, 4)
    }

    fn col_hints(&'a self) -> Self::ColHints {
        iter::repeat(TrackHints::default())
    }

    fn row_hints(&'a self) -> Self::RowHints {
        iter::repeat(TrackHints {
            fr_size: 1.0,
            ..TrackHints::default()
        })
    }

    fn get_hints(&self, id: ChildId) -> Option<WidgetHints> {
        match id {
            ChildId::Str("label") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 0),
                ..WidgetHints::default()
            }),
            ChildId::Str("bar") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 1),
                ..WidgetHints::default()
            }),
            ChildId::Str("slider") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 2),
                ..WidgetHints::default()
            }),
            ChildId::Str("nested_parent") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 3),
                ..WidgetHints::default()
            }),
            _ => None
        }
    }
}

impl<'a> GridLayout<'a> for NestedParentLayout {
    type ColHints = iter::Repeat<TrackHints>;
    type RowHints = iter::Repeat<TrackHints>;

    fn grid_size(&self) -> GridSize {
        GridSize::new(6, 1)
    }

    fn col_hints(&'a self) -> Self::ColHints {
        iter::repeat(TrackHints::default())
    }

    fn row_hints(&'a self) -> Self::RowHints {
        iter::repeat(TrackHints {
            fr_size: 1.0,
            ..TrackHints::default()
        })
    }

    fn get_hints(&self, id: ChildId) -> Option<WidgetHints> {
        match id {
            ChildId::Str("add_button") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 0),
                ..WidgetHints::default()
            }),
            ChildId::Str("del_button") => Some(WidgetHints {
                node_span: NodeSpan::new(1, 0),
                ..WidgetHints::default()
            }),
            ChildId::StrCollection("button_vec", num) => Some(WidgetHints {
                node_span: NodeSpan::new(2 + num, 0),
                ..WidgetHints::default()
            }),
            _ => None
        }
    }
}

fn main() {
    let mut window = Window::new(Group::new(BasicParent::new()), &WindowConfig::new());
    let mut button_buf = Vec::new();

    loop {
        let mut action = None;
        window.wait_actions(|new_act| {action = Some(new_act); false}).unwrap();
        match action.unwrap() {
            GalleryEvent::AddButton =>
                window.root.children_mut()
                      .nested_parent.children_mut()
                      .button_vec.push(TextButton::new(AddButton, "Another Button")),
            GalleryEvent::DelButton => {
                window.root.children_mut()
                      .nested_parent.children_mut()
                      .button_vec.pop().map(|button| button_buf.push(button));
            },
            GalleryEvent::SliderMoved(moved_to) =>
                window.root.children_mut()
                      .bar.status_mut().completion = Completion::Frac(moved_to as f32 / 128.0)
        }
    }
}
