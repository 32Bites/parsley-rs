use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut, RangeInclusive},
};

use super::{
    error::LexError,
    stream::{GraphemeLocation, Graphemes},
};

/// Trait that dictates whether a type is considered a token.
/// Used to avoid implementation conflicts.
pub trait TokenValue: Debug + Clone {
    /// Defines whether the lexer should avoid pushing the current token into the token
    /// list. This is useful for say whitespace.
    fn should_skip(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
/// Represents a lexical token.
/// It has an inclusive range dictating the grapheme indexes that the token was lexed from.
///
/// `TokenType` dictates the value of the token, and this is likely to be an enum provided by the developer.
///
/// It can be dereferenced into it's `TokenType` both mutably and immutably.
///
/// It also implements [std::convert::AsRef] and [std::convert::AsMut] for `TokenType`, and [RangeInclusive<usize>].
pub struct Token<TokenType: TokenValue> {
    range: RangeInclusive<usize>,
    // line: usize,
    // offset: usize,
    value: TokenType,
}

impl<TokenType: TokenValue> Display for Token<TokenType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
    pub fn new(
        token: TokenType,
        range: Option<RangeInclusive<usize>>, /*, line: usize, offset: usize*/
    ) -> Self {
        let range = match range {
            Some(range) => range,
            None => 0..=0,
        };

        Self {
            range,
            // line,
            // offset,
            value: token,
        }
    }

    /// Creates a token from a `TokenType` that has a range of `0..=0`, which is meaningless.
    pub fn from(token: TokenType) -> Self {
        Self::new(token, None /*, 0, 0*/)
    }

    /// Returns a reference to the token's value.
    pub fn token(&self) -> &TokenType {
        &self.value
    }

    /// Returns a mutable reference to the token's value.
    pub fn token_mut(&mut self) -> &mut TokenType {
        &mut self.value
    }

    /// Returns a reference to the range.
    ///
    /// If the range is `0..=0` the returned value will be `None`.
    pub fn range(&self) -> Option<&RangeInclusive<usize>> {
        if self.range == (0..=0) {
            None
        } else {
            Some(&self.range)
        }
    }

    /// Returns a mutable reference to the range.
    ///
    /// If the range is `0..=0` the returned value will be `None`.
    pub fn range_mut(&mut self) -> Option<&mut RangeInclusive<usize>> {
        if self.range == (0..=0) {
            None
        } else {
            Some(&mut self.range)
        }
    }

    /// Returns a reference to the range, without checking if it's equal to `0..=0`.
    pub fn range_raw(&self) -> &RangeInclusive<usize> {
        &self.range
    }

    /// Returns a mutable reference to the range, without checking if it's equal to `0..=0`.
    pub fn range_raw_mut(&mut self) -> &mut RangeInclusive<usize> {
        &mut self.range
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

impl<TokenType: TokenValue> AsRef<RangeInclusive<usize>> for Token<TokenType> {
    fn as_ref(&self) -> &RangeInclusive<usize> {
        &self.range
    }
}

impl<TokenType: TokenValue> AsMut<RangeInclusive<usize>> for Token<TokenType> {
    fn as_mut(&mut self) -> &mut RangeInclusive<usize> {
        &mut self.range
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
    fn lex(
        &mut self,
        tokens: &[Token<TokenType>],
        incoming: &mut Graphemes,
    ) -> Result<TokenType, LexError>;
}
