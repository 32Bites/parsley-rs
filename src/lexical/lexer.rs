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

impl<'a, TokenType: TokenValue + 'a> Lexer<'a, TokenType> {
    /// Create a lexer.
    pub fn new<Reader: SourceableReader<'a> + 'a>(
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

    /// Returns a reference to the lines that were read.
    pub fn lines(&self) -> &[String] {
        self.incoming.lines()
    }

    /// Returns the amount of lines that were read. Lines are deliminated by a LF, or CRLF.
    pub fn line_count(&self) -> usize {
        self.incoming.lines().len()
    }

    /// Returns the amount of graphemes read (but not peeked).
    pub fn graphemes(&self) -> usize {
        self.incoming.grapheme_count()
    }

    /// Returns the total invalid bytes read (not stored).\
    ///
    /// If the input is lossy, this is the amount of bytes that were read that were invalid, not the amount of bytes their utf-8 ('\u{FFFD}') representation occupies
    /// in memory.
    pub fn invalid_bytes(&self) -> usize {
        self.incoming.invalid_bytes()
    }

    /// Returns the total valid bytes read (not stored).
    pub fn valid_bytes(&self) -> usize {
        self.incoming.valid_bytes()
    }

    /// Returns the total bytes read (not stored).
    pub fn total_bytes(&self) -> usize {
        self.invalid_bytes() + self.valid_bytes()
    }

    /// Returns the bytes read from the input stream, replacing invalid bytes with '\u{FFFD}' if the input is lossy.
    ///
    /// Note, that this does include bytes that were peeked during lexing, and not yet officially "read".
    /// Do not use this to check how many bytes that were read but not peeked.
    pub fn bytes(&self) -> &[u8] {
        self.incoming.bytes()
    }

    // Tokenize until the input reader is unable to supply any new tokens, or until a lexical error occurs.
    pub fn tokenize(&mut self) -> Result<&[Token<TokenType>], LexError<'a>> {
        while let Some(result) = self.next() {
            match result {
                Ok(_) => {}
                Err(error) => return Err(error),
            }
        }

        return Ok(&self.tokens);
    }
}

impl<'a, TokenType: TokenValue + 'a> Iterator for Lexer<'a, TokenType> {
    type Item = Result<Option<Token<TokenType>>, LexError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // Get the next grapheme
        let (grapheme, location) = match self.incoming.next() {
            // Grapheme gotten successfully
            Some(Ok(gl)) => gl,
            // There was an error when getting the grapheme
            Some(Err(error)) => return Some(Err(error)),
            // No more graphemes, stream closed
            None => {
                return match self.eof_token.take() {
                    // Eof token is set, add it to the tokens vec and return a clone of it
                    Some(eof) => {
                        self.tokens.push(Token::from(eof));
                        return Some(Ok(self.tokens.last().cloned()));
                    }
                    // Eof token either does not exist or it was consumed on the last iteration.
                    None => None,
                };
            }
        };

        // Lookahead for next grapheme
        let next = self.incoming.peek().map(|(g, _)| g);
        self.incoming.reset_peek();

        // Prepare token data
        let mut token_data: Option<(usize, usize, usize, usize, Result<TokenType, LexError<'a>>)> =
            None;
        // Find a tokenizer for the current grapheme, and upon finding one, lex a token.
        for f in &self.creation_funcs {
            let mut tokenizer = f();
            if tokenizer.can_tokenize(&self.tokens, &grapheme, &location, &next) {
                // Store the starting location
                let start_index = location.index;
                let start_byte = *location.byte_range.start();
                let start_line = location.line;
                let start_column = location.column;
                let token = tokenizer.lex(&mut self.tokens, &mut self.incoming);
                self.incoming.reset_peek();

                // Set the token data
                token_data = Some((start_index, start_byte, start_line, start_column, token));
                break;
            }
        }

        // A token was found
        if let Some((start_index, start_byte, start_line, start_column, token)) = token_data {
            let token = match token {
                // Lexed successfully
                Ok(token) => token,
                // There was an issue when lexing
                Err(error) => return Some(Err(error)),
            };

            // Check if the token shouldn't be skipped
            if !token.should_skip() {
                // Store the end location of the token
                let end_index = self.incoming.current_index();
                let end_byte = self.incoming.current_byte_index();
                let end_line = self.incoming.current_line();
                let end_column = self.incoming.current_column();
                // Create a span for the token
                let span = Span::new(
                    &self.incoming.lines(),
                    start_index..=end_index,
                    start_byte..=end_byte,
                    start_line..=end_line,
                    start_column..=end_column,
                    &self.incoming,
                );

                // Create and store the token with a span
                let token = Token::new(token, span);
                self.tokens.push(token);

                // Return a clone of the token
                Some(Ok(self.tokens.last().cloned()))
            } else {
                // Return a success but with no token value, as it should be skipped.
                Some(Ok(None))
            }
        } else {
            // No token was found, return error
            Some(Err(LexError::other(format!(
                "Failed to find a tokenizer for {:?}",
                grapheme
            ))))
        }
    }
}
