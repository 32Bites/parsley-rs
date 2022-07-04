use std::{
    cell::RefCell,
    io::{BufRead, Cursor, Seek},
    rc::Rc,
};

use character_stream::{CharacterIterator, CharacterStream, CharacterStreamResult};

use super::{error::LexError, AnyToken, Token, EOF};

pub struct Lexer {
    tokens: Vec<Rc<RefCell<dyn AnyToken>>>,
    current_token: Option<Rc<RefCell<dyn AnyToken>>>,
    create_tokens:
        Vec<fn(index: usize, character: char) -> Result<Rc<RefCell<dyn AnyToken>>, LexError>>,
}

impl Lexer {
    /// Create a new Lexer.
    /// `input` is the input string to analyze.
    /// `new_token_functions` is the list of new token functions, usually from a NewTokenBuilder.
    pub fn new() -> Self {
        Self {
            tokens: vec![],
            create_tokens: vec![],
            current_token: None,
        }
    }

    pub fn add_token_creation(
        &mut self,
        func: fn(index: usize, character: char) -> Result<Rc<RefCell<dyn AnyToken>>, LexError>,
    ) {
        self.create_tokens.push(func)
    }

    /// Returns an immutable reference to the tokens.
    pub fn tokens(&self) -> &Vec<Rc<RefCell<dyn AnyToken>>> {
        &self.tokens
    }

    /// Returns a mutable reference to the tokens.
    pub fn tokens_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn AnyToken>>> {
        &mut self.tokens
    }

    /// Consumes self and returns the token list.
    pub fn take_tokens(self) -> Vec<Rc<RefCell<dyn AnyToken>>> {
        self.tokens
    }

    /// Create a new Token and replace the current Token with it.
    /// Return the previous Token for use elsewhere.
    /// Will return an error is there is a failure lexing with all provided new token functions.
    /// For internal use only.
    fn new_token(
        &mut self,
        index: usize,
        character: char,
    ) -> Result<Option<Rc<RefCell<dyn AnyToken>>>, LexError> {
        for func in &self.create_tokens {
            if let Ok(token) = func(index, character) {
                token.borrow_mut().lex(index, character)?;
                if let Some(old_token) = self.current_token.replace(token) {
                    return Ok(Some(old_token.clone()));
                } else {
                    return Ok(None);
                }
            }
        }

        Err(LexError::Character(
            "Failed to find a tokenizer for the current token".into(),
        ))
    }

    pub fn tokenize_from_stream<Reader: BufRead + Seek>(
        &mut self,
        reader: Reader,
        is_lossy: bool,
    ) -> Result<(), LexError> {
        let mut chars = CharacterIterator::new(CharacterStream::new(reader, is_lossy))
            .enumerate()
            .peekable();
        while let Some((index, character)) = chars.next() {
            let mut old_token: Option<Rc<RefCell<dyn AnyToken>>> = None;

            let character = match character {
                CharacterStreamResult::Character(character) => character,
                CharacterStreamResult::Failure(_, _) if is_lossy => unreachable!(),
                CharacterStreamResult::Failure(bytes, error) => {
                    return Err(LexError::CharacterRead(bytes, error))
                }
            };

            if let Some(current_token) = self.current_token.clone() {
                if current_token.borrow().is_done() {
                    // The current token is done lexing.
                    // Create a token!
                    if let Some(token) = self.new_token(index, character)? {
                        if !token.borrow().should_skip() {
                            old_token = Some(token.clone());
                        }
                    }
                } else {
                    // Continue to lex this token.
                    let error = current_token.borrow_mut().lex(index, character);
                    if let Err(error) = error {
                        if let LexError::StartNewToken(reuse_character) = error {
                            // TODO: Start new token
                            if reuse_character {
                                // Create a token!
                                if let Some(token) = self.new_token(index, character)? {
                                    if !token.borrow().should_skip() {
                                        old_token = Some(token);
                                    }
                                }
                            }
                        } else {
                            return Err(error);
                        }
                    }
                }
            } else {
                // We must be on the first token.
                // Create a token and lex it!
                // Create a token!
                if let Some(token) = self.new_token(index, character)? {
                    if !token.borrow().should_skip() {
                        old_token = Some(token);
                    }
                }
            }

            if let Some(old_token) = old_token {
                self.tokens.push(old_token);
            }

            if let None = chars.peek() {
                if let Some(current_token) = self.current_token.take() {
                    let borrowed = current_token.borrow();
                    if borrowed.is_done() || borrowed.can_be_forced() {
                        if !borrowed.should_skip() {
                            self.tokens.push(current_token.clone());
                        } // We dont want to crash if there is nothing to lex.
                    } else {
                        return Err(LexError::Other("Final token is not finished!".into()));
                    }
                }
            }
        }

        self.tokens.push(Rc::new(RefCell::new(Token::new(EOF))));

        Ok(())
    }

    /// Tokenize the current input, and return an error if there is a failure.
    /// If successful, the `tokens` field will hold the output.
    pub fn tokenize(&mut self, input: String) -> Result<(), LexError> {
        let cursor: Cursor<Vec<u8>> = Cursor::new(input.into_bytes());
        self.tokenize_from_stream(cursor, true)
    }
    /*
    /// Consumes self and creates a TreeBuilder for working attempting to parse an AST.
    pub fn tree_builder(self, allowed_patterns: SharedTreeBuilderPatternList) -> TreeBuilder {
        TreeBuilder::new(self, allowed_patterns)
    }*/
}
