pub mod widgets;

pub use dct::{geometry, events};
pub use dle::hints;
use self::hints::{WidgetHints, GridSize, TrackHints};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildId {
    Str(&'static str),
    Num(u32)
}

pub trait NodeProcessorInit: Sized {
    type Error;
    type GridProcessor: NodeProcessor<Error = Self::Error>;
    fn init_grid<C, R>(self, grid_size: GridSize, col_hints: C, row_hints: R) -> Self::GridProcessor
            where C: Iterator<Item = TrackHints>,
                  R: Iterator<Item = TrackHints>;
}

pub trait NodeProcessor: Sized {
    type Error;
}

pub trait NodeProcessorGrid<N: Node>: NodeProcessor {
    /// Add a child to the node processor.
    fn add_child<'a>(&'a mut self, ChildId, WidgetHints, node: &'a mut N) -> Result<(), Self::Error>;
}

pub trait NodeDataRegistry<N>
        where N: Node<Wrapper = Self::NodeDataWrapper>
{
    type NodeDataWrapper: NodeDataWrapper<N::Inner>;
}

pub trait Node {
    type Wrapper: NodeDataWrapper<Self::Inner>;
    type Inner;
    type Action;

    fn type_name(&self) -> &'static str;

    fn wrapper(&self) -> &Self::Wrapper;
    fn wrapper_mut(&mut self) -> &mut Self::Wrapper;
}

pub trait NodeDataWrapper<I> {
    fn from_node_data(I) -> Self;
    fn inner(&self) -> &I;
    fn inner_mut(&mut self) -> &mut I;
    fn unwrap(self) -> I;
}

pub trait Parent<NPI>
        where NPI: NodeProcessorInit
{
    type ChildAction;

    fn children(&mut self, NPI) -> Result<(), NPI::Error>;
}


impl NodeProcessor for ! {
    type Error = !;
}

impl NodeProcessorInit for ! {
    type Error = !;
    type GridProcessor = !;
    #[allow(unreachable_code)]
    fn init_grid<C, R>(self, _: GridSize, _: C, _: R) -> Self::GridProcessor
            where C: Iterator<Item = TrackHints>,
                  R: Iterator<Item = TrackHints>
    {match self {}}
}

impl<N: Node> NodeProcessorGrid<N> for ! {
    fn add_child<'a>(&'a mut self, _: ChildId, _: WidgetHints, _: &'a mut N) -> Result<(), !> {match *self {}}
}

impl NodeProcessor for () {
    type Error = !;
}

impl NodeProcessorInit for () {
    type Error = !;
    type GridProcessor = ();
    fn init_grid<C, R>(self, _: GridSize, _: C, _: R) -> Self::GridProcessor
            where C: Iterator<Item = TrackHints>,
                  R: Iterator<Item = TrackHints>
    {()}
}

impl<N: Node> NodeProcessorGrid<N> for () {
    fn add_child<'a>(&'a mut self, _: ChildId, _: WidgetHints, _: &'a mut N) -> Result<(), !> {Ok(())}
}

