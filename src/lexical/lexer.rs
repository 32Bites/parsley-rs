use std::{
    fmt::Debug,
    io::{BufRead, Seek},
};

use itertools::{Itertools, MultiPeek};
use unicode_reader::Graphemes;

use super::{error::LexError, stream::Chars, Tokenizer};

/// Represents a function that creates an empty token. This assumes that each token is represented by a single type,
/// such as an enum, however for each enumeration that will be used in the lexer, there is a corresponding `TokenizerFn`.
pub type TokenizerFn<Token, Reader> = fn() -> Box<dyn Tokenizer<Token, Reader>>;

/// Accepts graphemes from an input reader, and lexes them into tokens.
pub struct Lexer<
    Token: Debug + Clone,
    Reader: BufRead + Seek,
    Incoming: Iterator<Item = std::io::Result<char>> = Chars<Reader>,
> {
    tokens: Vec<Token>,
    creation_funcs: Vec<TokenizerFn<Token, Reader>>,
    eof_token: Option<Token>,
    incoming: MultiPeek<Graphemes<Incoming>>,
}

impl<Token: Debug + Clone, Reader: BufRead + Seek> Lexer<Token, Reader> {
    /// Create a lexer.
    pub fn new(reader: Reader, is_lossy: bool, eof_token: Option<Token>) -> Self {
        Self {
            tokens: vec![],
            creation_funcs: vec![],
            incoming: Graphemes::from(Chars::new(reader, is_lossy)).multipeek(),
            eof_token,
        }
    }

    /// Add a tokenizer function.
    pub fn add_tokenizer(&mut self, creation_func: TokenizerFn<Token, Reader>) {
        self.creation_funcs.push(creation_func)
    }

    /// Add a tokenizer function and return self.
    pub fn tokenizer(mut self, creation_func: TokenizerFn<Token, Reader>) -> Self {
        self.add_tokenizer(creation_func);
        self
    }

    /// Return the stored tokens.
    pub fn tokens(&self) -> &Vec<Token> {
        &self.tokens
    }

    /// Tokenize tokens and store them in self.
    pub fn tokenize(&mut self) -> Result<(), LexError> {
        while let Some(result) = self.incoming.next() {
            match result {
                Ok(grapheme) => {
                    let next = match self.incoming.peek() {
                        None => None,
                        Some(result) => match result {
                            Err(_) => None,
                            Ok(grapheme) => Some(grapheme.clone()),
                        },
                    };
                    match self
                        .creation_funcs
                        .iter()
                        .filter_map(|creation_func| {
                            let mut tokenizer = creation_func();
                            if tokenizer.can_tokenize(&self.tokens, &grapheme, &next) {
                                Some(tokenizer.lex(&self.tokens, &mut self.incoming))
                            } else {
                                None
                            }
                        })
                        .last()
                    {
                        Some(token) => self.tokens.push(token?),
                        None => {
                            return Err(LexError::other(format!(
                                "Failed to find tokenizer for {:?}",
                                grapheme
                            )))
                        }
                    }
                }
                Err(error) => return Err(LexError::other(error)),
            }
        }

        if let Some(eof_token) = &self.eof_token {
            self.tokens.push(eof_token.clone());
        }

        Ok(())
    }
}
