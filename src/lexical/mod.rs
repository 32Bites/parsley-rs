//mod lexer;
//mod token;
mod lexer;
mod stream;
mod token;

pub use lexer::*;
pub use token::*;

/// Stores error types.
pub mod error;

#[cfg(test)]
mod tests {
    use itertools::MultiPeek;
    use std::{fmt::Display, io::Cursor};
    use unicode_reader::Graphemes;

    use super::{error::LexError, stream::Chars, *};

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

    struct DoubleQuotedStringLexer {
        internal_value: String,
    }

    impl DoubleQuotedStringLexer {
        fn new() -> Self {
            Self {
                internal_value: "".into(),
            }
        }
    }

    impl<'a> Tokenizer<Token, Cursor<&'a str>> for DoubleQuotedStringLexer {
        fn can_tokenize(
            &mut self,
            _: &[Token],
            character: &str,
            next_character: &Option<String>,
        ) -> bool {
            if let ("\"", Some(next_g)) = (character, next_character) {
                if !matches!(next_g.as_str(), "\n" | "\r") {
                    return true;
                }
            }
            false
        }

        fn lex(
            &mut self,
            _: &[Token],
            incoming_characters: &mut MultiPeek<Graphemes<Chars<Cursor<&'a str>>>>,
        ) -> Result<Token, LexError> {
            if let Some('"') = self.internal_value.chars().last() {
                return Ok(Token::double_quoted_string(""));
            }
            loop {
                let mut character = match incoming_characters.next() {
                    Some(Ok(character)) => character,
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
            is_whitespace && character.is_whitespace()
        }
    }

    impl<'a> Tokenizer<Token, Cursor<&'a str>> for Whitespace {
        fn can_tokenize(&mut self, _: &[Token], grapheme: &str, _next: &Option<String>) -> bool {
            grapheme.chars().fold(true, Whitespace::is)
        }

        fn lex(
            &mut self,
            _: &[Token],
            incoming: &mut MultiPeek<Graphemes<Chars<Cursor<&'a str>>>>,
        ) -> Result<Token, LexError> {
            if let Some(Ok(first_grapheme)) = incoming.peek() {
                if !first_grapheme.chars().fold(true, Whitespace::is) {
                    return Ok(Token::Whitespace);
                }
                incoming.next();
            }
            loop {
                match incoming.next() {
                    Some(Ok(grapheme)) if grapheme.chars().fold(true, Whitespace::is) => {
                        match incoming.peek() {
                            Some(Ok(next_grapheme))
                                if !next_grapheme.chars().fold(true, Whitespace::is) =>
                            {
                                break
                            }
                            Some(_) => continue,
                            None => break,
                        }
                    }
                    Some(Ok(_)) => unreachable!(),
                    Some(Err(error)) => return Err(LexError::other(error)),
                    None => break,
                }
            }

            Ok(Token::Whitespace)
        }
    }
    #[test]
    fn test_lexer() {
        let input = r#""BHi\t\""   
        
                        "PPPps\n\n""#;
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
    }
}
