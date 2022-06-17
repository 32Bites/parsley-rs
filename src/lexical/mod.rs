mod lexer;
mod token;

pub use lexer::*;
pub use token::*;

/// Stores error types.
pub mod error;

#[macro_export]
macro_rules! empty_create {
    () => {
        || Some($crate::lexical::Token::new(Self::new()).any_token())
    };
}
