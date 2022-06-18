use crate::parsing::{SharedTreeBuilderPatternList, TreeBuilder};

use super::{error::LexCharacterError, token::AnyToken, TokenTypeFunctionality};

/// Alias for closures that return a new Token.
pub type LexerNewTokenFn = fn() -> Option<Box<dyn AnyToken>>;

/// Makes building a new token function list easier.
pub struct NewTokenBuilder {
    pub functions: Vec<LexerNewTokenFn>,
}

impl NewTokenBuilder {
    /// Create a new, empty NewTokenBuilder.
    pub fn new() -> Self {
        Self { functions: vec![] }
    }

    /// Create a new NewTokenBuilder with the first type `T`.
    pub fn create<T: TokenTypeFunctionality>() -> Self {
        Self::new().add_consume::<T>()
    }

    /// Add type `T`'s new token function.
    pub fn add<T: TokenTypeFunctionality>(&mut self) {
        self.functions.push(T::create());
    }

    /// Add type `T`'s new token function, and return self.
    pub fn add_consume<T: TokenTypeFunctionality>(mut self) -> Self {
        self.add::<T>();
        self
    }

    /// Build the function list.
    pub fn build(self) -> Vec<LexerNewTokenFn> {
        self.functions
    }
}

/// Performs basic lexical analysis.
pub struct Lexer {
    input: String,
    tokens: Vec<Box<dyn AnyToken>>,
    current_token: Option<Box<dyn AnyToken>>,
    new_token_functions: Vec<LexerNewTokenFn>,
}

impl Lexer {
    /// Create a new Lexer.
    /// `input` is the input string to analyze.
    /// `new_token_functions` is the list of new token functions, usually from a NewTokenBuilder.
    pub fn new(input: String, new_token_functions: Vec<LexerNewTokenFn>) -> Self {
        Self {
            tokens: vec![],
            current_token: None,
            input,
            new_token_functions,
        }
    }

    /// Returns an immutable reference to the tokens.
    pub fn tokens(&self) -> &Vec<Box<dyn AnyToken>> {
        &self.tokens
    }

    /// Returns a mutable reference to the tokens.
    pub fn tokens_mut(&mut self) -> &mut Vec<Box<dyn AnyToken>> {
        &mut self.tokens
    }

    /// Consumes self and returns the token list.
    pub fn take_tokens(self) -> Vec<Box<dyn AnyToken>> {
        self.tokens
    }

    /// Return a copy of the character following the provided index, if it exists.
    /// Internal use only.
    fn next_character(&self, index: usize) -> Option<char> {
        self.input.chars().nth(index + 1)
    }

    /// Create a new Token and replace the current Token with it.
    /// Return the previous Token for use elsewhere.
    /// Will return an error is there is a failure lexing with all provided new token functions.
    /// For internal use only.
    fn new_token(
        current_token: &mut Option<Box<dyn AnyToken>>,
        functions: &Vec<LexerNewTokenFn>,
        next_character: Option<char>,
        index: usize,
        character: char,
    ) -> Result<Option<Box<dyn AnyToken>>, LexCharacterError> {
        for new_func in functions {
            if let Some(mut token) = new_func() {
                if let Ok(_) = token.lex(index, character, next_character) {
                    if let Some(old_token) = current_token.replace(token) {
                        return Ok(Some(old_token));
                    } else {
                        return Ok(None);
                    }
                }
            }
        }

        Err(LexCharacterError::Other(format!(
            "Failed to find a token type that will accept the current character: '{}'",
            character
        )))
    }

    /// Tokenize the current input, and return an error if there is a failure.
    /// If successful, the `tokens` field will hold the output.
    pub fn tokenize(&mut self) -> Result<(), LexCharacterError> {
        for (index, character) in self.input.chars().enumerate() {
            let next_character = self.next_character(index);
            let mut old_token: Option<Box<dyn AnyToken>> = None;
            if let None = &self.current_token {
                let old = Self::new_token(
                    &mut self.current_token,
                    &self.new_token_functions,
                    next_character,
                    index,
                    character,
                );
                old_token = match old {
                    Ok(old) => old,
                    Err(error) => return Err(error),
                };
            } else if let Some(ref mut current_token) = self.current_token {
                if !current_token.is_done() {
                    if let Err(error) = current_token.lex(index, character, next_character) {
                        if let LexCharacterError::StartNewToken { reuse_character } = error {
                            if reuse_character {
                                let old = Self::new_token(
                                    &mut self.current_token,
                                    &self.new_token_functions,
                                    next_character,
                                    index,
                                    character,
                                );
                                old_token = match old {
                                    Ok(old) => old,
                                    Err(error) => return Err(error),
                                };
                            }
                        } else {
                            return Err(error);
                        }
                    }
                } else {
                    let old = Self::new_token(
                        &mut self.current_token,
                        &self.new_token_functions,
                        next_character,
                        index,
                        character,
                    );
                    old_token = match old {
                        Ok(old) => old,
                        Err(error) => return Err(error),
                    };
                }
            }

            if let Some(old_token) = old_token {
                self.tokens.push(old_token);
            }
        }

        if let Some(current_token) = self.current_token.take() {
            if current_token.is_done() {
                self.tokens.push(current_token);
            } else {
                return Err(LexCharacterError::Other(
                    "Failed when lexing the final token, it is unfinished.".into(),
                ));
            }
        }

        Ok(())
    }

    /// Consumes self and creates a TreeBuilder for working attempting to parse an AST.
    pub fn tree_builder(self, allowed_patterns: SharedTreeBuilderPatternList) -> TreeBuilder {
        TreeBuilder::new(self, allowed_patterns)
    }
}
