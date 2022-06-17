/// Module that stores types and methods for lexical analysis.
pub mod lexical;

#[cfg(test)]
mod tests {
    use crate::{lexical::{
        error::LexCharacterError,
        {Lexer, LexerNewTokenFn, NewTokenBuilder, ToAnyToken, Token, TokenType},
    }, empty_create};

    #[test]
    fn test_lexer_creation() {
        #[derive(Debug, Clone)]
        struct NumberToken;

        impl TokenType for NumberToken {
            fn new() -> Self {
                Self {}
            }

            fn is_done_func() -> crate::lexical::TokenIsDoneFn<Self> {
                |_, _| None
            }

            fn lex_func() -> crate::lexical::TokenLexFn<Self> {
                |internal_value, _value_store, _index, character, _next_character| {
                    if matches!(character, '0'..='9') {
                        internal_value.push(character)
                    } else {
                        return Err(LexCharacterError::StartNewToken {
                            reuse_character: true,
                        });
                    }
                    Ok(())
                }
            }

            fn create() -> LexerNewTokenFn {
                empty_create!()
            }
        }

        #[derive(Debug, Clone)]
        enum Operation {
            Addition,
            Subtraction,
            OpeningParan,
            ClosingParan
        }

        #[derive(Debug, Clone)]
        struct OperationToken(Operation);

        impl TokenType for OperationToken {
            fn new() -> Self {
                Self(Operation::Addition)
            }

            fn is_done_func() -> crate::lexical::TokenIsDoneFn<Self> {
                |_, _| None
            }

            fn lex_func() -> crate::lexical::TokenLexFn<Self> {
                |internal_value, value_store, _index, character, _next_character| {
                    if internal_value.len() == 1 || !matches!(character, '+' | '-' | '(' | ')') {
                        return Err(LexCharacterError::StartNewToken {
                            reuse_character: true,
                        });
                    }

                    internal_value.push(character);

                    match character {
                        '-' => value_store.0 = Operation::Subtraction,
                        '(' => value_store.0 = Operation::OpeningParan,
                        ')' => value_store.0 = Operation::ClosingParan,
                        '+' => {}
                        _ => unreachable!(),
                    }

                    Ok(())
                }
            }

            fn create() -> LexerNewTokenFn {
                empty_create!()
            }
        }

        let functions = NewTokenBuilder::create::<NumberToken>()
            .add_consume::<OperationToken>()
            .build();

        let mut lexer = Lexer::new("123+145-78(".into(), functions);
        lexer.tokenize().unwrap();

        for token in &lexer.tokens {
            println!("Token: {}", token)
        }
    }
}