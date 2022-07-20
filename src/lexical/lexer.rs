use std::io::Read;

use super::{error::LexError, stream::Graphemes, Token, TokenValue, Tokenizer};

/// Represents a function that creates an empty token. This assumes that each token is represented by a single type,
/// such as an enum, however for each enumeration that will be used in the lexer, there is a corresponding `TokenizerFn`.
pub type TokenizerFn<TokenType> = fn() -> Box<dyn Tokenizer<TokenType>>;

/// Accepts graphemes from an input reader, and lexes them into tokens.
pub struct Lexer<'a, TokenType: TokenValue> {
    tokens: Vec<Token<TokenType>>,
    creation_funcs: Vec<TokenizerFn<TokenType>>,
    eof_token: Option<TokenType>,
    incoming: Graphemes<'a>,
}

impl<'a, TokenType: TokenValue> Lexer<'a, TokenType> {
    /// Create a lexer.
    pub fn new<Reader: Read + 'a>(
        reader: Reader,
        is_lossy: bool,
        eof_token: Option<TokenType>,
    ) -> Self {
        Self {
            tokens: vec![],
            creation_funcs: vec![],
            incoming: Graphemes::new(reader, is_lossy),
            eof_token,
        }
    }

    /// Add a tokenizer function.
    pub fn add_tokenizer(&mut self, creation_func: TokenizerFn<TokenType>) {
        self.creation_funcs.push(creation_func)
    }

    /// Add a tokenizer function and return self.
    pub fn tokenizer(mut self, creation_func: TokenizerFn<TokenType>) -> Self {
        self.add_tokenizer(creation_func);
        self
    }

    /// Return the stored tokens.
    pub fn tokens(&self) -> &Vec<Token<TokenType>> {
        &self.tokens
    }

    /// Tokenize tokens and store them in self.
    pub fn tokenize(&mut self) -> Result<(), LexError> {
        while let Some(result) = self.incoming.next() {
            match result {
                Ok((location, grapheme)) => {
                    let next = match self.incoming.peek() {
                        None => None,
                        Some((_, result)) => match result {
                            Err(_) => None,
                            Ok(grapheme) => Some(grapheme.clone()),
                        },
                    };
                    self.incoming.reset_peek();

                    let mut found = false;

                    match self
                        .creation_funcs
                        .iter()
                        .filter_map(|creation_func| {
                            if !found {
                                let mut tokenizer = creation_func();
                                if tokenizer.can_tokenize(&self.tokens, &grapheme, &location, &next)
                                {
                                    let start_index = self.incoming.current_index();
                                    let token = tokenizer.lex(&self.tokens, &mut self.incoming);
                                    self.incoming.reset_peek();
                                    found = true;
                                    return Some((start_index, token));
                                }
                            }

                            None
                        })
                        .last()
                    {
                        Some((start_index, token)) => {
                            let token = token?;
                            let end_index = self.incoming.current_index();
                            let bounded_token = Token::new(token, Some(start_index..=end_index));

                            self.tokens.push(bounded_token)
                        }
                        None => {
                            return Err(LexError::other(format!(
                                "Failed to find tokenizer for {:?}",
                                grapheme
                            )))
                        }
                    }
                }
                Err((index, error)) => return Err(LexError::other_indexed(index, error)),
            }
        }

        if let Some(eof_token) = &self.eof_token {
            self.tokens.push(Token::from(eof_token.clone()));
        }

        Ok(())
    }

    pub fn lines(&self) -> usize {
        self.incoming.lines()
    }

    pub fn graphemes(&self) -> usize {
        self.incoming.successes()
    }

    pub fn dropped_bytes(&mut self) -> usize {
        self.incoming.invalid_bytes()
    }
}
