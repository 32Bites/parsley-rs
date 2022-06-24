use std::{cell::RefCell, rc::Rc};

use crate::lexical::{Lexer, TokenIter};

use super::{combinators::SharedParserFn, node::AnyNode};

/// Defines a structure that will build an AST.
pub struct TreeBuilder<'a> {
    pub root_parser: SharedParserFn<'a>,
}

impl<'a> TreeBuilder<'a> {
    /// Create a new TreeBuilder
    pub fn new(root_parser: SharedParserFn<'a>) -> Self {
        Self { root_parser }
    }

    /// Takes the output from a lexer and parses it.
    pub fn parse(&mut self, lexer: Lexer) -> Result<Rc<RefCell<dyn AnyNode>>, String> {
        let tokens = lexer.take_tokens();
        let tokens = Rc::new(RefCell::new(TokenIter::new(tokens)));
        let results = (self.root_parser.borrow())(tokens.clone());
        if tokens.borrow().len() > 0 {
            for tok in tokens.borrow_mut().clone() {
                println!("Failed: {}", tok.borrow().token_name());
            }
            return Err("Unhandled Tokens!".into());
        }

        results
    }
}
