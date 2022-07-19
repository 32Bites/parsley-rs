use std::error::Error;

#[derive(Debug)]
/// Represents an error that occurs when lexing.
pub enum LexError {
    /// An error that you can throw when a token requires that within it's lexical logic,
    /// the stream must not cease to return graphemes.
    UnexpectedEndOfStream,
    /// An error that simply holds a boxed error.
    Other(Box<dyn Error>),
    /// Same as [Self::Other], except with an accompanying index
    /// representing the location of the failed grapheme.
    OtherIndexed(usize, Box<dyn Error>),
}

impl LexError {
    /// Helper for creating a [LexError::Other].
    pub fn other<T: Into<Box<dyn Error>>>(error: T) -> Self {
        Self::Other(error.into())
    }

    /// Helper for creating a [LexError::OtherIndexed].
    pub fn other_indexed<T: Into<Box<dyn Error>>>(index: usize, error: T) -> Self {
        Self::OtherIndexed(index, error.into())
    }
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnexpectedEndOfStream => write!(
                f,
                "Encountered an unexpected EOF when reading graphemes for lexing."
            ),
            LexError::Other(error) => write!(f, "{}", error),
            LexError::OtherIndexed(index, error) => write!(
                f,
                "Error lexing the grapheme at index: {}. The error: {}",
                index, error
            ),
        }
    }
}

impl Error for LexError {}
