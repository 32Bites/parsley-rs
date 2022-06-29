// REDO COMBINATORS!!!!!!!!!

use std::{any::Any, cell::RefCell, fmt::Debug, rc::Rc};

use crate::lexical::{Token, TokenIter, TokenValue, EOF};

use super::node::{AnyNode, GetAnyNodeValue, NoValue, Node};

/// A result from parsing.
type ParserResult = Result<Rc<RefCell<dyn AnyNode>>, String>;

/// A shared reference to a parser function.
pub type SharedParserFn<'a> = Rc<RefCell<dyn Fn(Rc<RefCell<TokenIter>>) -> ParserResult + 'a>>;

/// Helper to make parser functions.
pub fn shared_parser<'a, P>(parser: P) -> SharedParserFn<'a>
where
    P: Fn(Rc<RefCell<TokenIter>>) -> ParserResult + 'a,
{
    Rc::new(RefCell::new(parser))
}

/// A parser builder that accepts a function `parser`.
/// `T` corresponds to the TokenValue that your desired token has.
/// `parser` is a function that will determine the validity of the token,
/// and return a node if valid.
/// `parser` must return a node or error.
pub fn token<'a, T: TokenValue>(parser: fn(Token<T>) -> ParserResult) -> SharedParserFn<'a> {
    shared_parser(move |tokens: Rc<RefCell<TokenIter>>| {
        let token = {
            let token = tokens.borrow_mut().next().ok_or("No next token!")?;
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
    })
}

/// A parser builder that returns a closure that will ensure that the next tokens will
/// follow the order of the parser functions in `parsers`.
/// `parsers` must be in the order in which you want the sequence to correspond to.
pub fn sequence<'a>(parsers: Vec<SharedParserFn<'a>>) -> SharedParserFn {
    shared_parser(move |tokens: Rc<RefCell<TokenIter>>| {
        // The results of each parser.
        let mut results: Vec<Rc<RefCell<dyn AnyNode>>> = vec![];
        // Loop through each parser.
        for parser in &parsers {
            // Parse with the remaining tokens, and store the resulting node if it was successful.
            let node = (parser.borrow())(tokens.clone())?;
            // Push the result if successful.
            results.push(node.clone());
        }

        // Create a node with no set type, but with the resulting nodes as children.
        let mut node = Node::<NoValue> {
            value: None,
            children: results,
        };

        // Return the node.
        Ok(node.any_node_shared())
    })
}

/// A parser builder that returns a closure that will "flatten" the sequence's resulting children.
/// The closure will then, return a node with a value of a vector of the children's values.
/// The children's values must be of type `T`, or `NoValue`.
/// If a child's value is `NoValue`, nothing will be pushed to the vector.
pub fn flatten<'a, T: Any + Debug>(sequence: SharedParserFn) -> SharedParserFn {
    shared_parser(move |tokens| {
        // Parse and get a sequence.
        let result = (sequence.borrow())(tokens)?;

        // A vector that will store the value within each child.
        let mut items: Vec<T> = vec![];

        // Get the children.
        let children = result.borrow();
        let children = children.children();

        // Loop through each child.
        for child in children {
            // Check if the current child is a node with a value of type T.
            if let Some(Node::<T> { value, children: _ }) = child.borrow_mut().value_mut() {
                // Take the value if it exists.
                if let Some(value) = value.take() {
                    // Push the taken value.
                    items.push(value);
                }
                // Next child.
                continue;
            }
            return Err("Failed when flattening, not all children match the provided type.".into());
        }

        // Create the new node
        let mut node = Node::<Vec<T>> {
            value: Some(items),
            children: vec![],
        };

        Ok(node.any_node_shared())
    })
}

/// Parser builder that returns a parser for EOF tokens.
pub fn eof<'a>() -> SharedParserFn<'a> {
    token::<'a, EOF>(|_| Ok(Rc::new(RefCell::new(Node::<NoValue>::empty()))))
}

/// Parser builder that ensures the following tokens follow any of the provided parsers in `parsers`.
/// Will return the node of the first successful parse procedure.
pub fn any_of<'a>(parsers: Vec<SharedParserFn<'a>>) -> SharedParserFn {
    if parsers.len() == 0 {
        panic!("No parsers were provided!")
    }

    shared_parser(move |tokens: Rc<RefCell<TokenIter>>| {
        let create_local = || Rc::new(RefCell::new(tokens.borrow().clone()));
        // Clone the tokens iterator so that we can operate on it without modifying the actual iterator.
        let mut local_tokens = create_local();
        println!("Tokens Len before the parsers: {}", tokens.borrow().len());
        // Loop through each parser
        for parser in &parsers {
            println!(
                "Local Tokens Len before the parser: {}",
                local_tokens.borrow().len()
            );
            // Execute the parser, and check if it succeeded.
            if let Ok(node) = (parser.borrow())(local_tokens.clone()) {
                // Take the iterator from local_tokens.
                println!(
                    "Local Tokens Len after the parser: {}",
                    local_tokens.borrow().len()
                );
                let local_tokens = local_tokens.take();
                // Replace the iterators in tokens.
                tokens.replace(local_tokens);
                // Return successfully.
                return Ok(node);
            }

            // Failed parsing, clone tokens again to prepare for the next parser.
            local_tokens = create_local();
        }

        println!("Tokens Len after the parsers: {}", tokens.borrow().len());

        // None of the parsers succeeded.
        Err("None of the parsers were able to conclude successfully!".into())
    })
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
        // Parsed nodes from the repeated parsers.
        let mut elements: Vec<Rc<RefCell<dyn AnyNode>>> = vec![];
        // Parser for each repeated item.
        let element = element.borrow();
        // Check if we're using a serparator
        if let Some(separator) = &separator {
            // We're using a separator!
            // The first tokens must be valid to the element parser.
            if let Ok(node) = element(tokens.clone()) {
                // It's valid, push it.
                elements.push(node);
            } else {
                // Invalid element, failure!!!
                return Err("Failed to tokenize parse element".into());
            }
            // Loop through for each separator.
            while let Ok(_) = (separator.borrow())(tokens.clone()) {
                // Check if the tokens following the current separator are indeed an element.
                if let Ok(node) = element(tokens.clone()) {
                    // Valid element, push it!
                    elements.push(node.clone());
                } else {
                    // Invalid Element, failure!
                    return Err("Failed to parse repeated element".into());
                }
            }
        } else {
            // No separator!
            // Loop through each valid elementr and push it.
            while let Ok(node) = element(tokens.clone()) {
                elements.push(node)
            }
        }

        /*
        let last_node = tokens.borrow().last_node();
        if let Some(last_node) = last_node {
            tokens.borrow_mut().push_front(last_node);
        }

        */

        let mut node: Node<NoValue> = Node {
            value: None,
            children: elements,
        };

        Ok(node.any_node_shared())
    }))
}

/// Parser builder that always succeeds.
pub fn nothing<'a>() -> SharedParserFn<'a> {
    Rc::new(RefCell::new(|_| {
        let mut node = Node::<NoValue>::empty();
        Ok(node.any_node_shared())
    }))
}
