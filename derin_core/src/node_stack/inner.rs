use std::mem;
use render::RenderFrame;
use tree::{Node, NodeIdent, NodeSummary, RootID, Update};

use cgmath::{EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{BoundBox, GeoBox};

// TODO: GET CODE REVIEWED FOR SAFETY

struct StackElement<'a, A, F: RenderFrame> {
    node: *mut (Node<A, F> + 'a),
    bounds: BoundBox<Point2<i32>>,
    index: usize
}

pub(crate) struct NRAllocCache<A, F: RenderFrame> {
    vec: Vec<StackElement<'static, A, F>>,
    ident_vec: Vec<NodeIdent>
}

pub struct NRVec<'a, A: 'a, F: 'a + RenderFrame> {
    cache: &'a mut Vec<StackElement<'static, A, F>>,
    vec: Vec<StackElement<'a, A, F>>,
    ident_vec: &'a mut Vec<NodeIdent>,
    top_parent_offset: Vector2<i32>,
    root_id: RootID
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodePath<'a, N: 'a + ?Sized> {
    pub node: &'a mut N,
    pub path: &'a [NodeIdent],
    pub top_parent_offset: Vector2<i32>
}

impl<A, F: RenderFrame> NRAllocCache<A, F> {
    pub fn new() -> NRAllocCache<A, F> {
        NRAllocCache {
            vec: Vec::new(),
            ident_vec: Vec::new()
        }
    }

    pub fn use_cache<'a>(&'a mut self, node: &mut (Node<A, F> + 'a), root_id: RootID) -> NRVec<'a, A, F> {
        let mut cache_swap = Vec::new();
        mem::swap(&mut cache_swap, &mut self.vec);

        let mut vec = unsafe {
            let (ptr, len, cap) = (cache_swap.as_ptr(), cache_swap.len(), cache_swap.capacity());
            mem::forget(cache_swap);
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<A, F>>(ptr), len, cap)
        };
        let ident_vec = &mut self.ident_vec;

        vec.push(StackElement {
            node: node,
            bounds: BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF),
            index: 0
        });
        ident_vec.push(NodeIdent::Num(0));

        NRVec {
            cache: &mut self.vec,
            vec, ident_vec,
            top_parent_offset: Vector2::new(0, 0),
            root_id
        }
    }
}

impl<'a, A, F: RenderFrame> NRVec<'a, A, F> {
    #[inline]
    pub fn top(&self) -> &(Node<A, F> + 'a) {
        self.vec.last().map(|n| unsafe{ &*n.node }).unwrap()
    }

    #[inline]
    pub fn top_mut(&mut self) -> NodePath<Node<A, F> + 'a> {
        NodePath {
            node: self.vec.last_mut().map(|n| unsafe{ &mut *n.node }).unwrap(),
            path: &self.ident_vec,
            top_parent_offset: self.top_parent_offset()
        }
    }

    #[inline]
    pub fn top_ident(&self) -> NodeIdent {
        *self.ident_vec.last().unwrap()
    }

    pub fn top_index(&self) -> usize {
        self.vec.last().unwrap().index
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        assert_ne!(0, len);
        for node_slice in self.vec[len-1..].windows(2).rev() {
            let parent = unsafe{ &*node_slice[0].node };
            let child = unsafe{ &*node_slice[1].node };

            if child.update_tag().needs_update(self.root_id) != Update::default() {
                parent.update_tag().mark_update_child_immutable();
            }
        }

        self.vec.truncate(len);
        self.ident_vec.truncate(len);

        self.top_parent_offset = Vector2::new(0, 0);
        for bounds in self.vec[..len-1].iter().map(|n| n.bounds) {
            self.top_parent_offset += bounds.min().to_vec();
        }
    }

    #[inline]
    pub fn top_parent_offset(&self) -> Vector2<i32> {
        self.top_parent_offset
    }

    #[inline]
    pub fn top_bounds_offset(&self) -> BoundBox<Point2<i32>> {
        self.top().bounds() + self.top_parent_offset
    }

    #[inline]
    pub fn nodes<'b>(&'b self) -> impl 'b + Iterator<Item=&'a Node<A, F>> + DoubleEndedIterator + ExactSizeIterator {
        self.vec.iter().map(|n| unsafe{ &*n.node })
    }

    #[inline]
    pub fn ident(&self) -> &[NodeIdent] {
        debug_assert_eq!(self.ident_vec.len(), self.vec.len());
        &self.ident_vec
    }

    #[inline]
    pub fn try_push<G>(&mut self, with_top: G) -> Option<NodeSummary<&'a mut Node<A, F>>>
        where G: FnOnce(&'a mut Node<A, F>, &[NodeIdent]) -> Option<NodeSummary<&'a mut Node<A, F>>>
    {
        let new_top_opt = with_top(unsafe{ mem::transmute(self.top_mut().node) }, &self.ident_vec );
        if let Some(new_top_summary) = new_top_opt {
            assert_ne!(new_top_summary.node as *mut Node<A, F>, self.top_mut().node as *mut _);
            {
                let cur_top = self.vec.last_mut().unwrap();

                cur_top.bounds = unsafe{ &*cur_top.node }.bounds();
                self.top_parent_offset += cur_top.bounds.min().to_vec();
            }

            self.vec.push(StackElement {
                node: new_top_summary.node,
                bounds: BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF),
                index: new_top_summary.index
            });
            self.ident_vec.push(new_top_summary.ident);
            Some(new_top_summary)
        } else {
            None
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&'a mut Node<A, F>> {
        // Ensure the base is never popped
        if self.vec.len() == 1 {
            return None;
        }

        let popped = self.vec.pop().map(|n| unsafe{ &mut *n.node }).unwrap();
        self.ident_vec.pop();
        let last_mut = self.vec.last_mut().unwrap();
        self.top_parent_offset -= last_mut.bounds.min().to_vec();
        last_mut.bounds = BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF);

        if popped.update_tag().needs_update(self.root_id) != Update::default() {
            self.top_mut().node.update_tag().mark_update_child_immutable();
        }


        Some(popped)
    }
}

impl<'a, A, F: RenderFrame> Drop for NRVec<'a, A, F> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
        self.vec.clear();
        self.ident_vec.clear();

        let mut vec = unsafe {
            let (ptr, len, cap) = (self.vec.as_ptr(), self.vec.len(), self.vec.capacity());
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<'static, A, F>>(ptr), len, cap)
        };
        let mut empty_vec = unsafe {
            let (ptr, len, cap) = (self.cache.as_ptr(), self.cache.len(), self.cache.capacity());
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<'a, A, F>>(ptr), len, cap)
        };

        mem::swap(self.cache, &mut vec);
        mem::swap(&mut self.vec, &mut empty_vec);

        mem::forget(vec);
        mem::forget(empty_vec);
    }
}

