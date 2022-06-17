mod lexer;
mod token;

pub use token::*;
pub use lexer::*;

/// Stores error types.
pub mod error;

#[macro_export]
macro_rules! empty_create {
    () => {
        || Some($crate::lexical::Token::new(Self::new()).any_token())
    };
}