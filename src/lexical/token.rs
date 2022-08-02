use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use super::{
    error::LexError,
    stream::{GraphemeLocation, Graphemes},
    Span,
};

/// Trait that dictates whether a type is considered a token.
/// Used to avoid implementation conflicts.
pub trait TokenValue: Debug + Clone + Display {
    /// Defines whether the lexer should avoid pushing the current token into the token
    /// list. This is useful for say whitespace.
    fn should_skip(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
/// Represents a lexical token.
/// It has a [super::Span] representing it's location in the stream.
///
/// `TokenType` dictates the value of the token, and this is likely to be an enum provided by the developer.
///
/// It can be dereferenced into it's `TokenType` both mutably and immutably.
///
/// It also implements [std::convert::AsRef] and [std::convert::AsMut] for `TokenType`, and [RangeInclusive<usize>].
pub struct Token<TokenType: TokenValue> {
    span: Span,
    value: TokenType,
}

impl<TokenType: TokenValue + Display> Display for Token<TokenType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Token {{")?;
        writeln!(f, "\tSpan: {{")?;
        writeln!(f, "\t{}", textwrap::indent(&self.span.to_string(), "\t"))?;
        writeln!(f, "\t}};")?;
        writeln!(
            f,
            "\tValue: {}",
            textwrap::indent(&self.value.to_string(), "\t")
        )?;
        writeln!(f, "}};")
    }
}

impl<TokenType: TokenValue + PartialEq> PartialEq for Token<TokenType> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<TokenType: TokenValue> Token<TokenType> {
    /// Creates a new token with the provided `token` and `range`.
    ///
    /// `token` is a `TokenType` that will contain the actual value of the token.
    ///
    /// `range` is an [Option] for a [std::ops::RangeInclusive<usize>]. If `range` is `None`, then the internal range will be `0..=0`.
    pub fn new(token: TokenType, span: Span) -> Self {
        Self {
            span,
            value: token,
        }
    }

    /// Creates a token from a `TokenType` that has a range of `0..=0`, which is meaningless.
    pub fn from(token: TokenType) -> Self {
        Self::new(token, Span::default())
    }

    /// Returns a reference to the token's value.
    pub fn token(&self) -> &TokenType {
        &self.value
    }

    /// Returns a mutable reference to the token's value.
    pub fn token_mut(&mut self) -> &mut TokenType {
        &mut self.value
    }

    /// Returns a reference to the token's [super::Span].
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns a mutable reference to the token's [super::Span].
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl<TokenType: TokenValue> Deref for Token<TokenType> {
    type Target = TokenType;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<TokenType: TokenValue> DerefMut for Token<TokenType> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<TokenType: TokenValue> AsRef<TokenType> for Token<TokenType> {
    fn as_ref(&self) -> &TokenType {
        &self.value
    }
}

impl<TokenType: TokenValue> AsMut<TokenType> for Token<TokenType> {
    fn as_mut(&mut self) -> &mut TokenType {
        &mut self.value
    }
}

impl<TokenType: TokenValue> AsRef<Span> for Token<TokenType> {
    fn as_ref(&self) -> &Span {
        &self.span
    }
}

impl<TokenType: TokenValue> AsMut<Span> for Token<TokenType> {
    fn as_mut(&mut self) -> &mut Span {
        &mut self.span
    }
}

/// Represents a tokenizer.
pub trait Tokenizer<TokenType: TokenValue> {
    /// Determines whether or not the given grapheme and potential next grapheme consitutes the start
    /// of a potentially valid token. If it is indeed valid and you require the current grapheme,
    /// store `grapheme` somewhere in your tokenizer. However do not store `next`, as it will be
    /// handled in the [lex](Self::lex) function.
    fn can_tokenize(
        &mut self,
        tokens: &[Token<TokenType>],
        grapheme: &str,
        grapheme_location: &GraphemeLocation,
        next: &Option<String>,
    ) -> bool;
    /// Given [can_tokenize](Sel::can_tokenize) evaluates to `true`, this function is called.
    ///
    /// It provides access to an immutable reference to the previous tokens, `tokens`.
    ///
    /// It also provides access to a mutable reference to the incoming stream, `incoming`.
    /// This stream is a stream of Unicode graphemes, from an underlying UTF-8 stream.
    /// Meaning rather than relying on singular characters, which doesn't include items
    /// such as emojis.
    fn lex<'a, 'b>(
        &'b mut self,
        tokens: &'b mut Vec<Token<TokenType>>,
        incoming: &'b mut Graphemes<'a>,
    ) -> Result<TokenType, LexError<'a>>;
}
