use std::{any::Any, cell::RefCell, rc::Rc};

use crate::lexical::{Token, TokenIter, TokenValue, EOF};

use super::node::{AnyNode, GetAnyNodeValue, NoValue, Node};

/// A result from parsing.
type ParserResult = Result<Rc<RefCell<dyn AnyNode>>, String>;

/// A shared reference to a parser function.
pub type SharedParserFn<'a> = Rc<RefCell<dyn Fn(Rc<RefCell<TokenIter>>) -> ParserResult + 'a>>;

/// A parser builder that accepts a function `parser`.
/// `T` corresponds to the TokenValue that your desired token has.
/// `parser` is a function that will determine the validity of the token,
/// and return a node if valid.
/// `parser` must return a node or error.
pub fn token<'a, T: TokenValue>(parser: fn(Token<T>) -> ParserResult) -> SharedParserFn<'a> {
    Rc::new(RefCell::new(move |tokens: Rc<RefCell<TokenIter>>| {
        let token = {
            let token = tokens.clone().borrow_mut().next().ok_or("No next token!")?;
            let token = token.borrow();
            let token = token.as_any().downcast_ref::<Token<T>>();
            token.map(|x| x.clone())
        };
        if let Some(value) = token {
            let node = (parser)(value)?;
            Ok(node)
        } else {
            Err("Token type does not match what is expected!".into())
        }
    }))
}

/// A parser builder that returns a closure that will ensure that the next tokens will
/// follow the order of the parser functions in `parsers`.
/// `parsers` must be in the order in which you want the sequence to correspond to.
pub fn sequence<'a>(parsers: Vec<SharedParserFn<'a>>) -> SharedParserFn {
    Rc::new(RefCell::new(move |tokens: Rc<RefCell<TokenIter>>| {
        let mut results: Vec<Rc<RefCell<dyn AnyNode>>> = vec![];
        for parser in &parsers {
            let node = (parser.borrow())(tokens.clone())?;
            results.push(node.clone());
        }

        let mut node = Node::<NoValue> {
            value: None,
            children: results,
        };

        Ok(node.any_node_shared())
    }))
}

/// A parser builder that returns a closure that will "flatten" the sequence's resulting children.
/// The closure will then, return a node with a value of a vector of the children's values.
/// The children's values must be of type `T`, or `NoValue`.
/// If a child's value is `NoValue`, nothing will be pushed to the vector.
pub fn flatten<'a, T: Any>(sequence: SharedParserFn) -> SharedParserFn {
    Rc::new(RefCell::new(move |tokens| {
        let result = (sequence.borrow())(tokens)?;

        let mut items: Vec<T> = vec![];

        for item in result.borrow().children() {
            if let Some(Node::<T> { value, children: _ }) = item.borrow_mut().value_mut() {
                if let Some(value) = value.take() {
                    items.push(value);
                    continue;
                } else {
                    return Err("".into());
                }
            } else if let Some(Node::<NoValue>{value: _, children: _}) = item.borrow_mut().value_mut() {
                continue;
            }
            return Err("".into());
        }

        let mut node = Node::<Vec<T>> {
            value: Some(items),
            children: vec![],
        };

        Ok(node.any_node_shared())
    }))
}

/// Parser builder that returns a parser for EOF tokens.
pub fn eof<'a>() -> SharedParserFn<'a> {
    token::<'a, EOF>(|_token| Ok(Rc::new(RefCell::new(Node::<NoValue>::empty()))))
}

/// Parser builder that ensures the following tokens follow any of the provided parsers in `parsers`.
/// Will return the node of the first successful parse procedure.
pub fn any_of<'a>(parsers: Vec<SharedParserFn<'a>>) -> SharedParserFn {
    if parsers.len() == 0 {
        panic!("No parsers were provided!")
    }

    Rc::new(RefCell::new(move |tokens: Rc<RefCell<TokenIter>>| {
        let new_iter = || tokens.clone();
        let mut local_tokens = new_iter();
        for parser in &parsers {
            if let Ok(node) = (parser.borrow())(local_tokens.clone()) {
                let old = local_tokens.take();
                tokens.replace(old);
                return Ok(node);
            }
            local_tokens = new_iter();
        }

        Err("".into())
    }))
}

/// Parser builder that defines a set of potentially repeated parsers.
/// Optionally, you can provide a `separator`, which will define a deliminator between
/// each consecutive element in the set of repeated parsers.
/// However, even when `separator` is not None, the final token
/// and by extension the only token if a single element is found,
/// will be omitted.
pub fn repeated<'a>(
    element: SharedParserFn<'a>,
    separator: Option<SharedParserFn<'a>>,
) -> SharedParserFn<'a> {
    Rc::new(RefCell::new(move |tokens: Rc<RefCell<TokenIter>>| {
        let mut elements: Vec<Rc<RefCell<dyn AnyNode>>> = vec![];
        let element = element.borrow();
        if let Some(separator) = &separator {
            //println!("Tokens Before: {:?}\n", tokens);
            if let Ok(node) = element(tokens.clone()) {
                elements.push(node);
            } else {
                return Err("Failed to tokenize parse element".into());
            }
            while let Ok(_) = (separator.borrow())(tokens.clone()) {
                if let Ok(node) = element(tokens.clone()) {
                    elements.push(node.clone());
                } else {
                    return Err("Failed to parse repeated element".into());
                }
            }
        } else {
            while let Ok(node) = element(tokens.clone()) {
                elements.push(node)
            }
        }

        let last_node = tokens.borrow().last_node();
        if let Some(last_node) = last_node {
            tokens.borrow_mut().push_front(last_node);
        }

        let mut node: Node<NoValue> = Node {
            value: None,
            children: elements,
        };

        Ok(node.any_node_shared())
    }))
}

/// Parser builder that always succeeds.
pub fn nothing<'a>() -> SharedParserFn<'a> {
    Rc::new(
        RefCell::new(
            |_| {
                let mut node = Node::<NoValue>::empty();
                Ok(node.any_node_shared())
            }
        )
    )
}