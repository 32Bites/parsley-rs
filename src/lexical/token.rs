use std::{
    fmt::Debug,
    io::{BufRead, Seek},
};

use itertools::MultiPeek;
use unicode_reader::Graphemes;

use super::{error::LexError, stream::Chars};

pub trait Tokenizer<Token: Debug, Reader: BufRead + Seek> {
    /// Determines whether or not the given grapheme and potential next grapheme consitutes the start
    /// of a potentially valid token. If it is indeed valid and you require the current grapheme,
    /// store `grapheme` somewhere in your tokenizer. However do not store `next`, as it will be
    /// handled in the [lex](Self::lex) function.
    fn can_tokenize(&mut self, tokens: &[Token], grapheme: &str, next: &Option<String>) -> bool;
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
        tokens: &[Token],
        incoming: &mut MultiPeek<Graphemes<Chars<Reader>>>,
    ) -> Result<Token, LexError>;
}
