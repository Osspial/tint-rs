// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::widget::{WidgetId, WidgetIdent, ROOT_IDENT};
use std::{
    cell::Cell,
    collections::{
        VecDeque,
        hash_map::{HashMap, Entry}
    },
    mem,
};
use fnv::FnvBuildHasher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WidgetInsertError {
    ParentNotInTree,
    /// Returned if we tried to insert a widget that's the root widget.
    ///
    /// This in bad because completing the operation would result in there being no root widget!
    WidgetIsRoot
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WidgetRelationError {
    WidgetNotFound,
    RelationNotFound
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WidgetTreeNode {
    parent_id: WidgetId,
    // If an entry in the child array is `None`, that means a high-index widget has been inserted
    // before it's lower-index counterparts.
    children: Vec<Option<WidgetId>>,
    data: WidgetData
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetData {
    pub ident: WidgetIdent,
    depth: Cell<u32>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VirtualWidgetTree {
    root: WidgetId,
    root_data: WidgetData,
    root_children: Vec<Option<WidgetId>>,
    tree_data: HashMap<WidgetId, WidgetTreeNode, FnvBuildHasher>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRevItem {
    pub ident: WidgetIdent,
    pub id: WidgetId,
}

impl VirtualWidgetTree {
    pub(crate) fn new(root: WidgetId) -> VirtualWidgetTree {
        VirtualWidgetTree {
            root,
            root_data: WidgetData {
                ident: ROOT_IDENT,
                depth: Cell::new(0)
            },
            root_children: Vec::new(),
            tree_data: HashMap::default()
        }
    }

    pub fn root_id(&self) -> WidgetId {
        self.root
    }

    /// Insert a widget ID into the tree. If the widget in already in the tree, change the widget's
    /// parent to the new parent.
    pub(crate) fn insert(&mut self, parent_id: WidgetId, widget_id: WidgetId, child_index: usize, widget_ident: WidgetIdent) -> Result<(), WidgetInsertError> {
        if widget_id == self.root {
            return Err(WidgetInsertError::WidgetIsRoot);
        }

        if let Some((parent_data, children)) = self.get_widget_node_mut(parent_id) {
            let parent_depth = parent_data.depth();

            crate::vec_remove_element(children, &Some(widget_id));
            if children.len() <= child_index {
                children.resize(child_index + 1, None);
            }
            let mut removed_widget_id = Some(widget_id);
            mem::swap(&mut removed_widget_id, &mut children[child_index]);

            match self.tree_data.entry(widget_id) {
                Entry::Occupied(mut occ) => {
                    let node = occ.get_mut();

                    let old_parent_id = node.parent_id;
                    node.parent_id = parent_id;
                    node.data.ident = widget_ident;

                    let (_, old_parent_children) = self.get_widget_node_mut(old_parent_id).expect("Bad tree state");
                    // Remove any trailing `None`s from the parent.
                    while let Some(None) = old_parent_children.last() {
                        old_parent_children.pop();
                    }

                    if old_parent_id != parent_id {
                        crate::vec_remove_element(old_parent_children, &Some(widget_id)).unwrap();
                        self.update_node_depth(parent_depth + 1, &self.tree_data[&widget_id]);
                    }
                },
                Entry::Vacant(vac) => {
                    vac.insert(WidgetTreeNode::new(parent_id, widget_ident, parent_depth + 1));
                }
            }
            if let Some(removed_widget) = removed_widget_id.filter(|id| *id != widget_id) {
                self.remove(removed_widget);
            }
            Ok(())
        } else {
            Err(WidgetInsertError::ParentNotInTree)
        }
    }

    fn update_node_depth(&self, depth: u32, node: &WidgetTreeNode) {
        node.data.depth.set(depth);
        for child_id in node.children.iter().cloned().flatten() {
            self.update_node_depth(depth + 1, &self.tree_data[&child_id]);
        }
    }

    pub(crate) fn remove(&mut self, widget_id: WidgetId) -> Option<WidgetData> {
        if let Entry::Occupied(occ) = self.tree_data.entry(widget_id) {
            let node = occ.remove();

            // Remove the widget from the parent's child list and remove any trailing `None`s.
            let mut parent_children = &mut self.get_widget_node_mut(node.parent_id).unwrap().1;
            crate::vec_remove_element(parent_children, &Some(widget_id));
            while let Some(None) = parent_children.last() {
                parent_children.pop();
            }

            // Remove all the child widgets.
            let mut widgets_to_remove = VecDeque::from(node.children);
            while let Some(remove_id) = widgets_to_remove.pop_front() {
                let remove_id = match remove_id {
                    Some(id) => id,
                    None => continue
                };
                let removed_node = match self.tree_data.entry(remove_id) {
                    Entry::Occupied(occ) => occ.remove(),
                    Entry::Vacant(_) => panic!("Bad tree state")
                };
                widgets_to_remove.extend(removed_node.children);
            }

            Some(node.data)
        } else {
            None
        }
    }

    // A recursive remove function existed at one point, but has been removed from the source tree.
    // Check commits from early January 2019 to find it.

    pub(crate) fn parent(&self, widget_id: WidgetId) -> Result<WidgetId, WidgetRelationError> {
        if widget_id == self.root {
            Err(WidgetRelationError::RelationNotFound)
        } else if let Some(node) = self.tree_data.get(&widget_id) {
            Ok(node.parent_id)
        } else {
            Err(WidgetRelationError::WidgetNotFound)
        }
    }

    pub(crate) fn sibling(&self, widget_id: WidgetId, offset: isize) -> Result<WidgetId, WidgetRelationError> {
        if widget_id == self.root {
            return if offset == 0 {
                Ok(self.root)
            } else {
                Err(WidgetRelationError::RelationNotFound)
            };
        }

        let node = self.tree_data.get(&widget_id).ok_or(WidgetRelationError::WidgetNotFound)?;

        // We have to do this check after getting the node so the proper error is returned if the
        // widget isn't in the tree.
        if offset == 0 {
            return Ok(widget_id);
        }

        let siblings = &self.get_widget_node(node.parent_id).unwrap().1;

        let sibling_index = crate::find_index(&siblings, &Some(widget_id)).unwrap() as isize + offset;
        siblings.get(sibling_index as usize).cloned().and_then(|id| id).ok_or(WidgetRelationError::RelationNotFound)
    }

    pub(crate) fn sibling_wrapping(&self, widget_id: WidgetId, offset: isize) -> Option<WidgetId> {
        if widget_id == self.root {
            return Some(self.root);
        }

        let node = self.tree_data.get(&widget_id)?;

        // We have to do this check after getting the node so the proper error is returned if the
        // widget isn't in the tree.
        if offset == 0 {
            return Some(widget_id);
        }

        let siblings = &self.get_widget_node(node.parent_id).unwrap().1;

        let mod_euc = |i, rhs| {
            let r = i % rhs;
            if r < 0 {
                if rhs < 0 {
                    r - rhs
                } else {
                    r + rhs
                }
            } else {
                r
            }
        };

        let sibling_index = crate::find_index(siblings, &Some(widget_id)).unwrap() as isize + offset;
        siblings[mod_euc(sibling_index, siblings.len() as isize) as usize]
    }

    pub(crate) fn child_index(&self, widget_id: WidgetId, child_index: usize) -> Result<WidgetId, WidgetRelationError> {
        let children = self.get_widget_node(widget_id).ok_or(WidgetRelationError::WidgetNotFound)?.1;

        children.get(child_index).cloned().and_then(|id| id).ok_or(WidgetRelationError::RelationNotFound)
    }

    pub(crate) fn child_ident(&self, widget_id: WidgetId, child_ident: WidgetIdent) -> Result<WidgetId, WidgetRelationError> {
        let mut children = self.children(widget_id).ok_or(WidgetRelationError::WidgetNotFound)?;

        children.find(|(_, data)| data.ident == child_ident)
            .map(|(id, _)| id)
            .ok_or(WidgetRelationError::RelationNotFound)
    }

    // pub(crate) fn child_from_end(&self, widget_id: WidgetId, offset: usize) -> Option<WidgetId> {unimplemented!()}

    pub(crate) fn children(&self, widget_id: WidgetId) -> Option<impl Iterator<Item=(WidgetId, &'_ WidgetData)>> {
        Some(self.children_nodes(widget_id)?.map(|(id, node)| (id, &node.data)))
    }

    fn children_nodes(&self, widget_id: WidgetId) -> Option<impl Iterator<Item=(WidgetId, &'_ WidgetTreeNode)>> {
        let (_, children) = self.get_widget_node(widget_id)?;
        Some(children.iter().flatten().map(move |c| (*c, self.tree_data.get(c).expect("Bad tree state"))))
    }

    pub fn all_nodes(&self) -> impl Iterator<Item=(WidgetId, &'_ WidgetData)> {
        Some((self.root, &self.root_data)).into_iter().chain(self.tree_data.iter().map(|(&k, v)| (k, &v.data)))
    }

    pub(crate) fn get_widget(&self, id: WidgetId) -> Option<&WidgetData> {
        self.get_widget_node(id).map(|(d, _)| d)
    }

    /// Returns `Option<WidgetData, Children>`
    fn get_widget_node(&self, id: WidgetId) -> Option<(&WidgetData, &[Option<WidgetId>])> {
        if self.root == id {
            Some((&self.root_data, &self.root_children))
        } else {
            self.tree_data.get(&id).map(|n| (&n.data, &n.children[..]))
        }
    }

    fn get_widget_node_mut(&mut self, id: WidgetId) -> Option<(&mut WidgetData, &mut Vec<Option<WidgetId>>)> {
        if self.root == id {
            Some((&mut self.root_data, &mut self.root_children))
        } else {
            self.tree_data.get_mut(&id).map(|n| (&mut n.data, &mut n.children))
        }
    }

    /// Gets the identifier chain of the widget, starting with the widget's identifier and ending
    /// with the root identifier.
    pub(crate) fn path_reversed(&self, id: WidgetId) -> Option<impl '_ + Iterator<Item=PathRevItem> + ExactSizeIterator> {
        struct ClosureIterator<F>(F, usize)
            where F: FnMut() -> Option<PathRevItem>;
        impl<F> Iterator for ClosureIterator<F>
            where F: FnMut() -> Option<PathRevItem>
        {
            type Item = PathRevItem;
            fn next(&mut self) -> Option<PathRevItem> {
                (self.0)()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.1, Some(self.1))
            }
        }
        impl<F> ExactSizeIterator for ClosureIterator<F>
            where F: FnMut() -> Option<PathRevItem> {}

        let get_widget_and_parent = move |id| {
            if self.root == id {
                Some((&self.root_data.ident, None, 1))
            } else if let Some(node) = self.tree_data.get(&id) {
                Some((&node.data.ident, Some(node.parent_id), node.data.depth() + 1))
            } else {
                None
            }
        };

        let mut finished = false;
        let mut id = id;
        let (mut ident, mut parent_id_opt, len) = get_widget_and_parent(id)?;
        Some(ClosureIterator(move || {
            if finished {
                return None;
            }

            let old_ident = ident;
            let old_id = id;
            if let Some(parent_id) = parent_id_opt {
                let (p_ident, p_id, _) = get_widget_and_parent(parent_id)?;
                ident = p_ident;
                parent_id_opt = p_id;
                id = parent_id;
            } else {
                finished = true;
            }
            Some(PathRevItem {
                ident: old_ident.clone(),
                id: old_id,
            })
        }, len as usize))
    }
}

impl WidgetTreeNode {
    fn new(parent_id: WidgetId, ident: WidgetIdent, depth: u32) -> WidgetTreeNode {
        WidgetTreeNode {
            parent_id,
            children: Vec::new(),
            data: WidgetData {
                ident,
                depth: Cell::new(depth)
            }
        }
    }
}

impl WidgetData {
    #[inline(always)]
    pub fn depth(&self) -> u32 {
        self.depth.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use derin_common_types::if_tokens;

    macro_rules! extract_virtual_tree_idents {
        ($(
            $root:ident $(in $old:ident)* $({$($rest:tt)*})*
        ),*) => {$(
            if_tokens!{($($old)*) {} else {
                let $root = WidgetId::new();
                println!("{} = {:?}", stringify!($root), $root);
            }}

            extract_virtual_tree_idents!{$($($rest)*)*}
        )*};
    }

    macro_rules! virtual_widget_tree {
        (
            let $tree_ident:pat = $root:ident $(in $old:ident)* $({$($rest:tt)*})*
        ) => {
            extract_virtual_tree_idents!{$root $(in $old)* $({$($rest)*})*}
            let $tree_ident = {
                #[allow(unused_mut)]
                {
                    let root_id = $root;
                    let mut tree = VirtualWidgetTree::new(root_id);
                    let mut rolling_index = 0;
                    virtual_widget_tree!(@insert root_id, tree, rolling_index, $($($rest)*)*);
                    let _ = rolling_index; // Silences warnings
                    tree
                }
            };
        };
        (
            @insert $parent:expr, $tree:expr, $index:ident,
            $($child:ident $(in $old:ident)* $({$($children:tt)*})*),*
        ) => {$({
            println!("insert {} {}", stringify!($child), $index);
            $tree.insert(
                $parent,
                $child,
                $index,
                WidgetIdent::Str(Arc::from(stringify!($child)))
            ).unwrap();
            $index += 1;


            $(
                let mut rolling_index = 0;
                virtual_widget_tree!(
                    @insert
                        $child,
                        $tree,
                        rolling_index,
                        $($children)*
                );
                let _ = rolling_index; // Silences warnings
            )*
        })*};
    }

    #[test]
    fn test_create_macro() {
        virtual_widget_tree!{
            let macro_tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1,
                child_2
            }
        };

        let mut manual_tree = VirtualWidgetTree::new(root);
        manual_tree.insert(root, child_0, 0, WidgetIdent::new_str("child_0")).unwrap();
        manual_tree.insert(root, child_1, 1, WidgetIdent::new_str("child_1")).unwrap();
        manual_tree.insert(child_0, child_0_1, 0, WidgetIdent::new_str("child_0_1")).unwrap();
        manual_tree.insert(root, child_2, 2, WidgetIdent::new_str("child_2")).unwrap();
        manual_tree.insert(child_0, child_0_2, 2, WidgetIdent::new_str("child_0_2")).unwrap();
        manual_tree.insert(child_0, child_0_3, 1, WidgetIdent::new_str("child_0_3")).unwrap();
        manual_tree.insert(child_0_2, child_0_2_0, 0, WidgetIdent::new_str("child_0_2_0")).unwrap();

        assert_eq!(manual_tree, macro_tree, "{:#?}\n!=\n{:#?}", manual_tree, macro_tree);
    }

    #[test]
    fn test_macro_in_old() {
        virtual_widget_tree!{
            let macro_tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1,
                child_2
            }
        };

        virtual_widget_tree!{
            let macro_tree_old = root in old {
                child_0 in old {
                    child_0_1 in old,
                    child_0_3 in old,
                    child_0_2 in old {
                        child_0_2_0 in old
                    }
                },
                child_1 in old,
                child_2 in old
            }
        };

        assert_eq!(macro_tree, macro_tree_old);
    }

    #[test]
    fn test_move() {
        virtual_widget_tree!{
            let mut tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };

        let child_1_ident = tree.get_widget(child_1).unwrap().ident.clone();
        tree.insert(child_0_1, child_1, 0, child_1_ident).unwrap();
        virtual_widget_tree!{
            let tree_moved = root in old {
                child_0 in old {
                    child_0_1 in old {
                        child_1 in old {
                            child_1_0 in old,
                            child_1_1 in old
                        }
                    },
                    child_0_3 in old,
                    child_0_2 in old {
                        child_0_2_0 in old
                    }
                },
                child_2 in old
            }
        };
        assert_eq!(tree, tree_moved, "{:#?}\n!=\n{:#?}", tree, tree_moved);
    }

    #[test]
    fn test_relations() {
        virtual_widget_tree!{
            let tree = root {
                child_0 {
                    child_0_1,
                    child_0_2 {
                        child_0_2_0
                    },
                    child_0_3
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };
        println!("{:#?}", tree);

        assert_eq!(Err(WidgetRelationError::WidgetNotFound), tree.parent(WidgetId::new()));
        assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.parent(root));
        assert_eq!(Ok(root), tree.parent(child_0));
        assert_eq!(Ok(root), tree.parent(child_1));
        assert_eq!(Ok(root), tree.parent(child_2));
        assert_eq!(Ok(child_0), tree.parent(child_0_1));
        assert_eq!(Ok(child_0), tree.parent(child_0_2));
        assert_eq!(Ok(child_0), tree.parent(child_0_3));
        assert_eq!(Ok(child_0_2), tree.parent(child_0_2_0));
        assert_eq!(Ok(child_1), tree.parent(child_1_0));
        assert_eq!(Ok(child_1), tree.parent(child_1_1));

        for i in -16..16 {
            assert_eq!(Err(WidgetRelationError::WidgetNotFound), tree.sibling(WidgetId::new(), i), "{}", i);
            assert_eq!(None, tree.sibling_wrapping(WidgetId::new(), i), "{}", i);
            if i != 0 {
                assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.sibling(root, i), "{}", i);
                assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.sibling(child_0_2_0, i), "{}", i);
            }
            assert_eq!(Some(root), tree.sibling_wrapping(root, i), "{}", i);
            assert_eq!(Some(child_0_2_0), tree.sibling_wrapping(child_0_2_0, i), "{}", i);
        }

        assert_eq!(10, tree.all_nodes().count());
        for (id, _) in tree.all_nodes() {
            assert_eq!(Ok(id), tree.sibling(id, 0));
            assert_eq!(Some(id), tree.sibling_wrapping(id, 0));
        }

        assert_eq!(Ok(child_1), tree.sibling(child_0, 1));
        assert_eq!(Ok(child_2), tree.sibling(child_0, 2));
        assert_eq!(Ok(child_0), tree.sibling(child_1, -1));
        assert_eq!(Ok(child_2), tree.sibling(child_1, 1));
        assert_eq!(Ok(child_0), tree.sibling(child_2, -2));
        assert_eq!(Ok(child_1), tree.sibling(child_2, -1));

        for i in (-15..15).filter(|i| i % 3 == 0) {
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_0, i - 2), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_0, i - 1), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_0, i + 0), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_0, i + 1), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_0, i + 2), "{}", i);

            assert_eq!(Some(child_2), tree.sibling_wrapping(child_1, i - 2), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_1, i - 1), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_1, i + 0), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_1, i + 1), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_1, i + 2), "{}", i);

            assert_eq!(Some(child_0), tree.sibling_wrapping(child_2, i - 2), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_2, i - 1), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_2, i + 0), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_2, i + 1), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_2, i + 2), "{}", i);
        }

        for i in 0..16 {
            assert_eq!(Err(WidgetRelationError::WidgetNotFound), tree.child_index(WidgetId::new(), i));
        }
        assert_eq!(Ok(child_0), tree.child_index(root, 0));
        assert_eq!(Ok(child_1), tree.child_index(root, 1));
        assert_eq!(Ok(child_2), tree.child_index(root, 2));
        assert_eq!(Ok(child_0_1), tree.child_index(child_0, 0));
        assert_eq!(Ok(child_0_2), tree.child_index(child_0, 1));
        assert_eq!(Ok(child_0_3), tree.child_index(child_0, 2));
        assert_eq!(Ok(child_0_2_0), tree.child_index(child_0_2, 0));
        assert_eq!(Ok(child_1_0), tree.child_index(child_1, 0));
        assert_eq!(Ok(child_1_1), tree.child_index(child_1, 1));
        assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.child_index(root, 3));
    }

    #[test]
    fn test_ident_chain() {
        virtual_widget_tree!{
            let tree = root {
                child_0 {
                    child_0_1,
                    child_0_2 {
                        child_0_2_0
                    },
                    child_0_3
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };

        macro_rules! ident_chain {
            ($($ident:ident),*) => {{
                vec![
                    $(PathRevItem {
                        id: $ident,
                        ident: if $ident == root {
                            ROOT_IDENT
                        } else {
                            WidgetIdent::new_str(stringify!($ident))
                        }
                    },)*
                ]
            }}
        }

        macro_rules! test_ident_chain {
            ($first:ident $(, $ident:ident)*) => {
                let iter = tree.path_reversed($first).unwrap();
                let path_ref = ident_chain!($first $(, $ident)*);
                assert_eq!(path_ref.len(), iter.len());
                assert_eq!(path_ref, iter.collect::<Vec<_>>());
            }
        }

        assert!(tree.path_reversed(WidgetId::new()).is_none());
        test_ident_chain![root];
        test_ident_chain![child_0, root];
        test_ident_chain![child_1, root];
        test_ident_chain![child_2, root];
        test_ident_chain![child_0_1, child_0, root];
        test_ident_chain![child_0_2, child_0, root];
        test_ident_chain![child_0_3, child_0, root];
        test_ident_chain![child_0_2_0, child_0_2, child_0, root];
        test_ident_chain![child_1_0, child_1, root];
        test_ident_chain![child_1_1, child_1, root];
    }

    #[test]
    fn test_depth() {
        virtual_widget_tree!{
            let mut tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };

        assert_eq!(Some(0), tree.get_widget(root).map(|w| w.depth()));
        assert_eq!(Some(1), tree.get_widget(child_0).map(|w| w.depth()));
        assert_eq!(Some(1), tree.get_widget(child_1).map(|w| w.depth()));
        assert_eq!(Some(1), tree.get_widget(child_2).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_0_1).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_0_2).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_0_3).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_1_0).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_1_1).map(|w| w.depth()));
        assert_eq!(Some(3), tree.get_widget(child_0_2_0).map(|w| w.depth()));

        let child_1_ident = tree.get_widget(child_1).unwrap().ident.clone();
        tree.insert(child_0_1, child_1, 0, child_1_ident).unwrap();
        virtual_widget_tree!{
            let tree_moved = root in old {
                child_0 in old {
                    child_0_1 in old {
                        child_1 in old {
                            child_1_0 in old,
                            child_1_1 in old
                        }
                    },
                    child_0_3 in old,
                    child_0_2 in old {
                        child_0_2_0 in old
                    }
                },
                child_2 in old
            }
        };
        assert_eq!(tree, tree_moved, "{:#?}\n!=\n{:#?}", tree, tree_moved);

        assert_eq!(Some(0), tree.get_widget(root).map(|w| w.depth()));
        assert_eq!(Some(1), tree.get_widget(child_0).map(|w| w.depth()));
        assert_eq!(Some(3), tree.get_widget(child_1).map(|w| w.depth()));
        assert_eq!(Some(1), tree.get_widget(child_2).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_0_1).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_0_2).map(|w| w.depth()));
        assert_eq!(Some(2), tree.get_widget(child_0_3).map(|w| w.depth()));
        assert_eq!(Some(4), tree.get_widget(child_1_0).map(|w| w.depth()));
        assert_eq!(Some(4), tree.get_widget(child_1_1).map(|w| w.depth()));
        assert_eq!(Some(3), tree.get_widget(child_0_2_0).map(|w| w.depth()));
    }

    #[test]
    fn test_duplicate_insert() {
        virtual_widget_tree!{
            let mut macro_tree = root {
                child_0
            }
        };
        let reference_tree = macro_tree.clone();
        println!("tree created");

        macro_tree.insert(root, child_0, 0, WidgetIdent::new_str("child_0")).unwrap();
        macro_tree.insert(root, child_0, 0, WidgetIdent::new_str("child_0")).unwrap();
        macro_tree.insert(root, child_0, 0, WidgetIdent::new_str("child_0")).unwrap();

        assert_eq!(macro_tree, reference_tree);
    }

    #[test]
    fn widget_move() {
        virtual_widget_tree!{
            let mut macro_tree = root {
                child_0,
                child_1,
                child_2
            }
        };

        macro_tree.insert(root, child_1, 0, WidgetIdent::new_str("child_1")).unwrap();

        virtual_widget_tree!{
            let expected_tree = root in old {
                child_1 in old,
                child_2 in old
            }
        };

        assert_eq!(macro_tree, expected_tree);
    }

    #[test]
    fn widget_trim() {
        virtual_widget_tree!{
            let mut macro_tree = root {
                child_0
            }
        };
        let child_1 = WidgetId::new();
        let reference_tree = macro_tree.clone();

        macro_tree.insert(root, child_1, 10, WidgetIdent::new_str("child_1")).unwrap();
        macro_tree.remove(child_1);
        assert_eq!(macro_tree, reference_tree);

        virtual_widget_tree!{
            let reference_tree = root in old {
                child_0 in old,
                child_1 in old
            }
        };

        macro_tree.insert(root, child_1, 10, WidgetIdent::new_str("child_1")).unwrap();
        macro_tree.insert(root, child_1, 1, WidgetIdent::new_str("child_1")).unwrap();
        assert_eq!(macro_tree, reference_tree);
    }
}
