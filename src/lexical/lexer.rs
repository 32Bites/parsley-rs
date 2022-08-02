use super::{
    error::LexError, stream::Graphemes, SourceableReader, Span, Token, TokenValue, Tokenizer,
};

/// Represents a function that creates an empty token. This assumes that each token is represented by a single type,
/// such as an enum, however for each enumeration that will be used in the lexer, there is a corresponding `TokenizerFn`.
pub trait TokenizerFn<'a, TokenType: TokenValue>:
    Fn() -> Box<dyn Tokenizer<TokenType> + 'a> + 'a
{
}

impl<'a, TokenType: TokenValue, T: Fn() -> Box<dyn Tokenizer<TokenType> + 'a> + 'a>
    TokenizerFn<'a, TokenType> for T
{
}

/// Accepts graphemes from an input reader, and lexes them into tokens.
pub struct Lexer<'a, TokenType: TokenValue> {
    tokens: Vec<Token<TokenType>>,
    creation_funcs: Vec<Box<dyn TokenizerFn<'a, TokenType>>>,
    eof_token: Option<TokenType>,
    pub(crate) incoming: Graphemes<'a>,
}

impl<'a, TokenType: TokenValue> Lexer<'a, TokenType> {
    /// Create a lexer.
    pub fn new<Reader: SourceableReader + 'a>(
        reader: Reader,
        is_lossy: bool,
        store_bytes: bool,
        eof_token: Option<TokenType>,
    ) -> Self {
        Self {
            tokens: vec![],
            creation_funcs: vec![],
            incoming: Graphemes::new(reader, is_lossy, !store_bytes),
            eof_token,
        }
    }

    /// Add a tokenizer function and return self.
    pub fn tokenizer<F, T>(mut self, f: F) -> Self
    where
        F: Fn() -> T + 'a,
        T: Tokenizer<TokenType> + 'a,
    {
        self.add_tokenizer(f);
        self
    }

    /// Add a tokenizer function.
    pub fn add_tokenizer<F, T>(&mut self, f: F)
    where
        F: Fn() -> T + 'a,
        T: Tokenizer<TokenType> + 'a,
    {
        self.creation_funcs.push(Box::new(move || Box::new(f())));
    }

    /// Return a reference to the tokens.
    pub fn tokens(&self) -> &Vec<Token<TokenType>> {
        &self.tokens
    }

    /// Return a mutable reference to the tokens.
    pub fn tokens_mut(&mut self) -> &mut Vec<Token<TokenType>> {
        &mut self.tokens
    }

    /// Return the tokens and consume `self`.
    pub fn take(self) -> Vec<Token<TokenType>> {
        self.tokens
    }

    /// Tokenize tokens and store them in self.
    pub fn tokenize(&mut self) -> Result<(), LexError<'a>> {
        while let Some(result) = self.incoming.next() {
            match result {
                Ok((grapheme, location)) => {
                    let next = match self.incoming.peek() {
                        None => None,
                        Some((grapheme, _)) => Some(grapheme),
                    };
                    self.incoming.reset_peek();

                    let mut found = false;

                    match self
                        .creation_funcs
                        .iter()
                        .filter_map(|creation_func: &Box<dyn TokenizerFn<'a, TokenType>>| {
                            if !found {
                                let mut tokenizer = creation_func();
                                if tokenizer.can_tokenize(&self.tokens, &grapheme, &location, &next)
                                {
                                    let start_index = location.index;
                                    let start_byte = *location.byte_range.start();
                                    let start_line = location.line;
                                    let start_line_column = location.column;
                                    let token = tokenizer.lex(&mut self.tokens, &mut self.incoming);
                                    self.incoming.reset_peek();
                                    found = true;
                                    return Some((
                                        start_index,
                                        start_byte,
                                        start_line,
                                        start_line_column,
                                        token,
                                    ));
                                }
                            }

                            None
                        })
                        .last()
                    {
                        Some((start_index, start_byte, start_line, start_line_offset, token)) => {
                            let token = token?;
                            if !token.should_skip() {
                                let end_index = self.incoming.current_index();
                                let end_byte = self.incoming.current_byte_index();
                                let end_line = self.incoming.current_line();
                                let end_line_column = self.incoming.current_column();
                                let span = Span::new(
                                    &self.incoming.lines(),
                                    start_index..=end_index,
                                    start_byte..=end_byte,
                                    start_line..=end_line,
                                    start_line_offset..=end_line_column,
                                    &self.incoming,
                                );
                                let token = Token::new(token, span);
                                self.tokens.push(token)
                            }
                        }
                        None => {
                            return Err(LexError::other(format!(
                                "Failed to find tokenizer for {:?}",
                                grapheme
                            )))
                        }
                    }
                }
                Err(error) => return Err(error),
            }
        }

        if let Some(eof_token) = &self.eof_token {
            self.tokens.push(Token::from(eof_token.clone()));
        }

        Ok(())
    }

    pub fn lines(&self) -> &[String] {
        self.incoming.lines()
    }

    pub fn line_count(&self) -> usize {
        self.incoming.lines().len()
    }

    pub fn graphemes(&self) -> usize {
        self.incoming.grapheme_count()
    }

    pub fn invalid_bytes(&self) -> usize {
        self.incoming.invalid_bytes()
    }

    pub fn valid_bytes(&self) -> usize {
        self.incoming.valid_bytes()
    }

    pub fn total_bytes(&self) -> usize {
        self.invalid_bytes() + self.valid_bytes()
    }

    pub fn bytes(&self) -> &[u8] {
        self.incoming.bytes()
    }
}
