use std::{any::Any, cell::RefCell, collections::VecDeque, rc::Rc};

use super::error::LexError;

/// Describes a kind of token.
pub trait TokenValue: Sized + std::fmt::Debug + 'static + Clone {
    /// Creates a new Self, if the index and character are valid.
    fn new(index: usize, character: char) -> Result<Self, LexError>;
    /// Creates a new token with a token type of Self, and returns a shared reference to it if successful.
    fn create_token(index: usize, character: char) -> Result<Rc<RefCell<dyn AnyToken>>, LexError> {
        let value_store = Self::new(index, character)?;
        Ok(Token::new(value_store).shared())
    }

    /// Returns the name of the token kind.
    fn token_name(&self) -> String;

    /// Lex a character at a given index.
    fn lex(
        &mut self,
        internal_value: &mut String,
        index: usize,
        character: char,
    ) -> Result<(), LexError>;

    /// Returns whether the token is done parsing.
    /// None means that it doesn't matter.
    fn is_done(&self) -> Option<bool>;

    /// Same as `is_done`, except it is only executed before the lexer finishes. 
    /// This is used as a failsafe for tokens that while in other contexts, would need
    /// more characters to lex, do not need to continue lexing when the next token is
    /// an EOF. Usually this entails a "default" of some kind for that token.
    fn can_be_forced(&self) -> Option<bool>;
    /// Determines whether this token type should avoid being
    /// pushed into the output of a lexer.
    fn should_skip(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
/// Describes a token.
/// `ValueStore` determines the type of token.
/// `value_store` is a value of type `ValueStore`, which stores the state of lexing.
/// `internal_value` stores the validated characters for this token.
pub struct Token<ValueStore: TokenValue> {
    pub internal_value: String,
    pub value_store: ValueStore,
}

impl<ValueStore: TokenValue> Token<ValueStore> {
    /// Create a new token with a.
    /// `value_store` determines the token type, and provides an empty state for
    /// lexing.
    pub fn new(value_store: ValueStore) -> Self {
        Self {
            internal_value: "".into(),
            value_store,
        }
    }

    /// Consume `self` and return a shared reference.
    pub fn shared(self) -> Rc<RefCell<dyn AnyToken>> {
        Rc::new(RefCell::new(self))
    }

    /// Lex character and index, returning an error if it was unsuccessful.
    pub fn lex(&mut self, index: usize, character: char) -> Result<(), LexError> {
        self.value_store
            .lex(&mut self.internal_value, index, character)
    }

    /// Return the token name.
    pub fn token_name(&self) -> String {
        self.value_store.token_name()
    }

    /// Returns whether this token be skipped in the lexer output.
    pub fn should_skip(&self) -> bool {
        self.value_store.should_skip()
    }

    /// Return whether or not this token is done lexing.
    pub fn is_done(&self) -> bool {
        self.value_store.is_done().unwrap_or(true)
    }

    /// Returns whether or not this token can be considered done at an EOF, despite `is_done` evaluating to false.
    fn can_be_forced(&self) -> bool {
        self.value_store.can_be_forced().unwrap_or(true)
    }
}

/// Trait to describe all tokens.
pub trait AnyToken: std::fmt::Debug + Any {
    /// Lex index and character.
    fn lex(&mut self, index: usize, character: char) -> Result<(), LexError>;
    /// Return the token's type name.
    fn token_name(&self) -> String;
    /// Returns whether this token should be skipped when appending to the output
    /// of a lexer.
    fn should_skip(&self) -> bool;
    /// Returns whether or not this token is done lexing.
    fn is_done(&self) -> bool;

    /// Returns whether or not this token can be considered done at an EOF, despite `is_done` evaluating to false.
    fn can_be_forced(&self) -> bool;

    /// Converts `self` to an Any reference.
    fn as_any(&self) -> &dyn Any;
    /// Same as `as_any`, but the reference is mutable.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<ValueStore: TokenValue> AnyToken for Token<ValueStore> {
    fn lex(&mut self, index: usize, character: char) -> Result<(), LexError> {
        self.lex(index, character)
    }

    fn should_skip(&self) -> bool {
        self.should_skip()
    }

    fn token_name(&self) -> String {
        self.token_name()
    }

    fn is_done(&self) -> bool {
        self.is_done()
    }

    fn can_be_forced(&self) -> bool {
        self.can_be_forced()
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

/// Trait to make checking AnyToken's real type easier.
pub trait AnyTokenCheck {
    /// Checks if self is a `Token<T>`.
    fn token_type_check<T: TokenValue>(&self) -> bool;
}

impl AnyTokenCheck for dyn AnyToken {
    fn token_type_check<T: TokenValue>(&self) -> bool {
        self.as_any().is::<Token<T>>()
    }
}

#[derive(Clone, Default, Debug)]
/// An iterator of tokens.
pub struct TokenIter {
    /// The tokens to be iterated over.
    tokens: VecDeque<Rc<RefCell<dyn AnyToken>>>,
    /// The last node to be popped from the front.
    last_node: Option<Rc<RefCell<dyn AnyToken>>>,
}

impl TokenIter {
    /// Creates a new iterator of tokens from a vector of shared tokens.
    pub fn new(tokens: Vec<Rc<RefCell<dyn AnyToken>>>) -> Self {
        Self {
            tokens: VecDeque::from(tokens),
            last_node: None,
        }
    }

    /// Returns the last node to be popped from the front.
    pub fn last_node(&self) -> Option<Rc<RefCell<dyn AnyToken>>> {
        self.last_node.clone()
    }

    /// Push a node to the front
    pub fn push_front(&mut self, node: Rc<RefCell<dyn AnyToken>>) {
        self.tokens.push_front(node)
    }
}

impl Iterator for TokenIter {
    type Item = Rc<RefCell<dyn AnyToken>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.last_node = self.tokens.front().map(|x| x.clone());
        self.tokens.pop_front()
    }
}

impl ExactSizeIterator for TokenIter {
    fn len(&self) -> usize {
        self.tokens.len()
    }
}

#[derive(Clone, Debug)]
/// EOF token.
pub struct EOF;

impl TokenValue for EOF {
    fn new(_index: usize, _character: char) -> Result<Self, LexError> {
        Err(LexError::Other(
            "An EOF token should not be denoted by a character or index.".into(),
        ))
    }

    fn token_name(&self) -> String {
        "EOF".into()
    }

    fn lex(
        &mut self,
        _internal_value: &mut String,
        _index: usize,
        _character: char,
    ) -> Result<(), LexError> {
        Err(LexError::Other("An EOF token cannot be lexed".into()))
    }

    fn is_done(&self) -> Option<bool> {
        None
    }

    fn can_be_forced(&self) -> Option<bool> {
        None
    }
}
