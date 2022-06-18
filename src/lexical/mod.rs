mod lexer;
mod token;

pub use lexer::*;
pub use token::*;

/// Stores error types.
pub mod error;

/// Empty create closure for TokenType.
#[macro_export]
macro_rules! empty_create {
    () => {{
        use $crate::lexical::ToAnyToken;
        || Some($crate::lexical::Token::new(Self::new()).any_token())
    }};
}
