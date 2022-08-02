use std::{error::Error, sync::Arc};

#[derive(Debug, Clone)]
/// Represents an error that occurs when lexing.
pub enum LexError<'a> {
    /// An error that you can throw when a token requires that within it's lexical logic,
    /// the stream must not cease to return graphemes.
    UnexpectedEndOfStream,
    /// An error that simply holds a boxed error.
    Other(Arc<dyn Error + 'a>),
}

impl<'a> LexError<'a> {
    /// Helper for creating a [LexError::Other].
    pub fn other<T: Into<Box<dyn Error + 'a>>>(error: T) -> Self {
        let boxed: Box<dyn Error + 'a> = error.into();
        Self::Other(boxed.into())
    }
}

impl std::fmt::Display for LexError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnexpectedEndOfStream => write!(
                f,
                "Encountered an unexpected EOF when reading graphemes for lexing."
            ),
            LexError::Other(error) => write!(f, "{}", error),
        }
    }
}

impl Error for LexError<'_> {}
