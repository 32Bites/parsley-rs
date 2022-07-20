//mod lexer;
//mod token;
mod lexer;
mod stream;
mod token;

pub use lexer::*;
pub use stream::*;
pub use token::*;

/// Stores error types.
pub mod error;

#[cfg(test)]
mod tests {
    use std::{fmt::Display, io::Cursor};

    use super::{error::LexError, *};

    #[derive(Debug, Clone)]
    enum Token {
        Eof,
        DoubleQuotedString(String),
        Whitespace,
    }

    impl Token {
        fn double_quoted_string<S: AsRef<str>>(string: S) -> Self {
            Token::DoubleQuotedString(string.as_ref().to_string())
        }
    }

    impl super::TokenValue for Token {
        fn should_skip(&self) -> bool {
            matches!(self, Self::Whitespace)
        }
    }

    impl Display for Token {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let string: String = match self {
                Token::Eof => "EOF".into(),
                Token::DoubleQuotedString(string) => {
                    format!("Quoted String := {:?}", string)
                }
                Token::Whitespace => "Whitespace".into(),
            };

            write!(f, "{}", string)
        }
    }

    #[derive(Default)]
    struct DoubleQuotedStringLexer {
        internal_value: String,
    }

    impl DoubleQuotedStringLexer {
        fn new() -> Self {
            Self::default()
        }
    }

    impl<'a> Tokenizer<Token> for DoubleQuotedStringLexer {
        fn can_tokenize(
            &mut self,
            _: &[super::Token<Token>],
            grapheme: &str,
            _: &super::stream::GraphemeLocation,
            next_grapheme: &Option<String>,
        ) -> bool {
            if let ("\"", Some(next_g)) = (grapheme, next_grapheme) {
                if !matches!(next_g.as_str(), "\n" | "\r") {
                    return true;
                }
            }
            false
        }

        fn lex(
            &mut self,
            _: &[super::Token<Token>],
            incoming_characters: &mut super::stream::Graphemes,
        ) -> Result<Token, LexError> {
            if let Some('"') = self.internal_value.chars().last() {
                return Ok(Token::double_quoted_string(""));
            }
            loop {
                let mut character = match incoming_characters.next() {
                    Some(Ok((_location, grapheme))) => grapheme,
                    Some(_) => unimplemented!(),
                    None => return Err(LexError::UnexpectedEndOfStream),
                };

                if let Some('\\') = self.internal_value.chars().last() {
                    character = match character.as_str() {
                        "r" => '\r',
                        "n" => '\n',
                        "t" => '\t',
                        "\"" => '\"',
                        _ => {
                            return Err(LexError::other(format!(
                                "Invalid escape character '\\{}'",
                                character
                            )))
                        }
                    }
                    .to_string();
                    self.internal_value.pop();
                } else {
                    if character == "\"" {
                        return Ok(Token::DoubleQuotedString(self.internal_value.clone()));
                    }
                }

                self.internal_value.push_str(&character)
            }
        }
    }

    struct Whitespace;

    impl Whitespace {
        fn is(is_whitespace: bool, character: char) -> bool {
            is_whitespace && (character.is_whitespace() || character == '\u{FFFD}')
        }
    }

    impl<'a> Tokenizer<Token> for Whitespace {
        fn can_tokenize(
            &mut self,
            _: &[super::Token<Token>],
            grapheme: &str,
            _: &super::stream::GraphemeLocation,
            _next: &Option<String>,
        ) -> bool {
            grapheme.chars().fold(true, Whitespace::is)
        }

        fn lex(
            &mut self,
            _: &[super::Token<Token>],
            incoming: &mut super::stream::Graphemes,
        ) -> Result<Token, LexError> {
            if let Some((_, Ok(first_grapheme))) = incoming.peek() {
                if !first_grapheme.chars().fold(true, Whitespace::is) {
                    return Ok(Token::Whitespace);
                }
                incoming.next();
            }
            loop {
                match incoming.next() {
                    Some(Ok((_, grapheme))) if grapheme.chars().fold(true, Whitespace::is) => {
                        match incoming.peek() {
                            Some((_, Ok(next_grapheme)))
                                if !next_grapheme.chars().fold(true, Whitespace::is) =>
                            {
                                break
                            }
                            Some(_) => continue,
                            None => break,
                        }
                    }
                    Some(Ok((_, _))) => unreachable!(),
                    Some(Err((index, error))) => return Err(LexError::other_indexed(index, error)),
                    None => break,
                }
            }

            Ok(Token::Whitespace)
        }
    }
    #[test]
    fn test_lexer() {
        let mut input = br#""BHi\t\""   
        
                        "PPPps\n\n""#
            .to_vec();
        for _ in 0..100 {
            input.push(0xAD);
        }
        let input = Cursor::new(input);

        let mut lexer = Lexer::new(input, true, Some(Token::Eof));
        lexer.add_tokenizer(|| Box::new(DoubleQuotedStringLexer::new()));
        lexer.add_tokenizer(|| Box::new(Whitespace));

        if let Err(error) = lexer.tokenize() {
            panic!("{}", error)
        }

        println!("Displaying the lexed tokens:");

        for token in lexer.tokens() {
            println!("{}", token)
        }

        println!("Invalid byte count: {}", lexer.dropped_bytes());
    }
}
