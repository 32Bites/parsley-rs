use std::any::Any;

use super::{error::LexCharacterError, lexer::LexerNewTokenFn};

/// A type alias for use in the TokenType trait. 
/// It represents the lex function in a Token, or what is run when attempting to lex a character.
pub type TokenLexFn<TokType> = fn(
    internal_value: &mut String,
    value_store: &mut TokType,
    index: usize,
    character: char,
    next_character: Option<char>,
) -> Result<(), LexCharacterError>;


/// A type alias for use in the TokenType trait.
/// It represents the function that is called on the final Token that was lexed.
/// It's purpose is to determin whether the final token has finished lexing, however
/// if this functionality is not needed, it should return None.
pub type TokenIsDoneFn<TokType> = fn(internal_value: &String, value_store: &TokType) -> Option<bool>;

/// Used to declare what kind of token a Token struct is, as well as serving as a value store
/// for more complicated token types.
pub trait TokenType: std::fmt::Debug + Clone {
    /// Returns the closure that is used for lexing a character.
    fn lex_func() -> TokenLexFn<Self>;
    /// Returns a closure that is used to check whether the token is done lexing.
    /// If None is returned, the Lexer will just ignore the function call entirely.
    /// The closure is called if the token is the last to be handled.
    fn is_done_func() -> TokenIsDoneFn<Self>;
    /// Returns a closure that can be used to create a new Token;
    fn create() -> LexerNewTokenFn;
    /// Creates a new `TokenType`.
    fn new() -> Self;
}

/// Used to represent a generalized token.
/// All tokens are a Token, but their behavior is defined by the provided `TokType`.
/// `TokType` is a TokenType that the token receives behavior from.
pub struct Token<TokType: TokenType> {
    internal_value: String,
    value_store: TokType,
    lex_func: TokenLexFn<TokType>,
    is_done_func: TokenIsDoneFn<TokType>
}

impl<TokType: TokenType> Token<TokType> {
    /// Creates a new Token that conforms to the TokenType `TokType`.
    pub fn new(value_store: TokType) -> Self {
        Self {
            internal_value: "".into(),
            value_store: value_store,
            lex_func: TokType::lex_func(),
            is_done_func: TokType::is_done_func()
        }
    }
    /// Returns the value store name string.
    /// For internal use only.
    fn value_store_string(&self) -> String {
        format!("{:?}", self.value_store)
    }

    /// For functions that need to inform the lexer whether they are finished parsing.
    /// This is useful for parsing tokens that must end with a character, or set of characters.
    /// Returns None if the potential result in the Option is to be ignored.
    fn is_done(&self) -> Option<bool> {
        (self.is_done_func)(&self.internal_value, &self.value_store)
    }

    /// Lexes the current character (`character`) at `index` to attempt to perform analysis.
    /// May use `next_character` for a lookahead.
    /// For internal use only.
    pub(crate) fn lex(
        &mut self,
        index: usize,
        character: char,
        next_character: Option<char>,
    ) -> Result<(), LexCharacterError> {
        (self.lex_func)(
            &mut self.internal_value,
            &mut self.value_store,
            index,
            character,
            next_character,
        )
    }
}

impl<TokType: TokenType> std::fmt::Display for Token<TokType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} - {{Internal Value: \"{}\"}}",
            self.value_store,
            self.internal_value.escape_default().to_string()
        )
    }
}

/// Used to transform an AnyToken into A token that conforms to `TokType`.
pub trait GetToken {
    /// Do as defined above.
    fn get_token<TokType: 'static + TokenType>(&self) -> Option<&Token<TokType>>;
    /// Do as defined above but mutably.
    fn get_token_mut<TokType: 'static + TokenType>(&mut self) -> Option<&mut Token<TokType>>;
}

impl GetToken for dyn AnyToken {
    fn get_token<TokType: 'static + TokenType>(&self) -> Option<&Token<TokType>> {
        self.as_any().downcast_ref::<Token<TokType>>()
    }

    fn get_token_mut<TokType: 'static + TokenType>(&mut self) -> Option<&mut Token<TokType>> {
        self.as_any_mut().downcast_mut::<Token<TokType>>()
    }
}

/// Used to represent all kinds of Tokens, as a "workaround" to Rusts' lack of type inheritance.
pub trait AnyToken: Any + std::fmt::Display {
    /// Lex the current token.
    /// For internal use only.
    fn lex(
        &mut self,
        index: usize,
        character: char,
        next_character: Option<char>,
    ) -> Result<(), LexCharacterError>;
    fn value_store(&self) -> String;
    fn is_done(&self) -> bool;

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<TokType: TokenType + 'static> AnyToken for Token<TokType> {
    fn value_store(&self) -> String {
        self.value_store_string()
    }

    fn is_done(&self) -> bool {
        self.is_done().unwrap_or(true)
    }

    fn lex(
        &mut self,
        index: usize,
        character: char,
        next_character: Option<char>,
    ) -> Result<(), LexCharacterError> {
        self.lex(index, character, next_character)
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

/// Converts self to AnyToken.
/// Consumes self.
pub trait ToAnyToken {
    fn any_token(self) -> Box<dyn AnyToken>;
}

impl<OriginalToken: AnyToken> ToAnyToken for OriginalToken {
    fn any_token(self) -> Box<dyn AnyToken> {
        Box::new(self)
    }
}
