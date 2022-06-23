use std::{any::Any, cell::RefCell, rc::Rc, vec};

/// Type to describe zero value.
pub struct NoValue;

/// Trait to describe all nodes.
pub trait AnyNode: Any {
    /// Convert the node to an Any reference.
    fn as_any(&self) -> &dyn Any;
    /// Convert the node to a mutable Any reference.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Return a reference to the node's children.
    fn children(&self) -> &Vec<Rc<RefCell<dyn AnyNode>>>;
    /// Convert `self` to an reference-counted, mutable, smart pointer.
    /// Consumes `self`.
    fn shared(self) -> Rc<RefCell<dyn AnyNode>>;
}

impl<T: Any> AnyNode for Node<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn shared(self) -> Rc<RefCell<dyn AnyNode>> {
        Rc::new(RefCell::new(self))
    }

    fn children(&self) -> &Vec<Rc<RefCell<dyn AnyNode>>> {
        self.children()
    }
}

/// Helper trait for checking a node's value against a type, and potentially getting it's value.
pub trait GetAnyNodeValue {
    /// Get an option containing a reference to a Node with a value type of `T`.
    fn value<T: Any>(&self) -> Option<&Node<T>>;
    /// Same as `value`, except the reference in the option is mutable.
    fn value_mut<T: Any>(&mut self) -> Option<&mut Node<T>>;

    /// Checks if `self` is a Node<T>.
    fn is_type<T: Any>(&self) -> bool;
}

impl GetAnyNodeValue for dyn AnyNode {
    fn value<T: Any>(&self) -> Option<&Node<T>> {
        self.as_any().downcast_ref::<Node<T>>()
    }

    fn value_mut<T: Any>(&mut self) -> Option<&mut Node<T>> {
        self.as_any_mut().downcast_mut::<Node<T>>()
    }

    fn is_type<T: Any>(&self) -> bool {
        self.as_any().is::<T>()
    }
}

// UNFINISHED
struct NodeIter {
    node: Rc<RefCell<dyn AnyNode>>,
    index: usize,
}

impl<'a> Iterator for NodeIter {
    type Item = Rc<RefCell<dyn AnyNode>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.node.borrow().children().get(self.index) {
            Some(child) => {
                self.index += 1;
                Some(child.clone())
            }
            None => None,
        }
    }
}

/// Describes a node in a tree.
/// `T` determines the value type of `value`.
/// `value` is the node's corresponding value.
/// `children` is a vector of node references, which
/// are the current node's children.
pub struct Node<T: Any> {
    pub value: Option<T>,
    pub children: Vec<Rc<RefCell<dyn AnyNode>>>,
}

impl<T: Any> Node<T> {
    /// Convert `self` to an AnyNode reference.
    pub fn any_node(&self) -> &dyn AnyNode {
        self as &dyn AnyNode
    }

    /// Same as any_node, but the reference is mutable.
    pub fn any_node_mut(&mut self) -> &mut dyn AnyNode {
        self as &mut dyn AnyNode
    }

    /// Same as any_node_mut, except the return value is a reference-counted, mutable, smart pointer.
    pub fn any_node_shared(&mut self) -> Rc<RefCell<dyn AnyNode>> {
        let old = std::mem::replace(self, Self::empty());
        Rc::new(RefCell::new(old))
    }

    /// Returns an empty node.
    pub fn empty() -> Self {
        Node::<T> {
            value: None,
            children: vec![],
        }
    }

    /// Return `self` and replace `self` with an empty node.
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Self::empty())
    }

    // UNFINISHED
    #[allow(dead_code)]
    fn iter(self) -> NodeIter {
        NodeIter {
            node: Rc::new(RefCell::new(self)),
            index: 0,
        }
    }

    /// Return the node's children.
    pub fn children(&self) -> &Vec<Rc<RefCell<dyn AnyNode>>> {
        &self.children
    }
}
