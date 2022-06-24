// TODO: Figure out how the fuck to write a parser that is portable.
// This will use my attempt at trying to understand parser combinators.

/// Provides cominators for easy parsing.
pub mod combinators;
mod node;
mod tree_builder;
// mod error;

pub use node::*;
pub use tree_builder::*;

/// Macros for parsing
pub mod macros {
    #[macro_export]
    macro_rules! sequence {
        ($($val:expr),+) => {
            $crate::parsing::combinators::sequence(vec![$($val),+])
        };
    }

    #[macro_export]
    macro_rules! token {
        ($val:expr) => {
            $crate::parsing::combinators::token($val);
        };
    }

    #[macro_export]
    macro_rules! flatten {
        ($id:ty, $val:expr) => {
            $crate::parsing::combinators::flatten::<$id>($val);
        };
    }

    #[macro_export]
    macro_rules! repeated {
        ($ele:expr, $sep:expr) => {
            $crate::parsing::combinators::repeated($ele, $sep);
        };

        ($ele:expr) => {
            $crate::repeated!($ele, None);
        };
    }

    #[macro_export]
    macro_rules! eof {
        () => {
            $crate::parsing::combinators::eof();
        };
    }

    #[macro_export]
    macro_rules! any_of {
        ($($val:expr),+) => {
            $crate::parsing::combinators::any_of(vec![$($val),+]);
        };
    }

    #[macro_export]
    macro_rules! nothing {
        () => {
            $crate::parsing::combinators::nothing();
        };
    }
}
