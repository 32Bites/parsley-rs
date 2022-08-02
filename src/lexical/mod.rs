//mod lexer;
//mod token;
mod lexer;
mod span;
mod stream;
mod token;

pub use lexer::*;
pub use span::*;
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

    impl Tokenizer<Token> for DoubleQuotedStringLexer {
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

        fn lex<'a, 'b>(
            &'b mut self,
            _: &'b mut Vec<super::Token<Token>>,
            incoming_characters: &'b mut super::stream::Graphemes<'a>,
        ) -> Result<Token, LexError<'a>> {
            loop {
                let mut character = match incoming_characters.next() {
                    Some(Ok((grapheme, _location))) => grapheme,
                    Some(Err(error)) => return Err(LexError::other(error)),
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

    impl Tokenizer<Token> for Whitespace {
        fn can_tokenize(
            &mut self,
            _: &[super::Token<Token>],
            grapheme: &str,
            _: &super::stream::GraphemeLocation,
            _next: &Option<String>,
        ) -> bool {
            grapheme.chars().fold(true, Whitespace::is)
        }

        fn lex<'a, 'b>(
            &'b mut self,
            _: &'b mut Vec<super::Token<Token>>,
            incoming: &'b mut super::stream::Graphemes<'a>,
        ) -> Result<Token, LexError<'a>> {
            if let Some((first_grapheme, _)) = incoming.peek() {
                if !first_grapheme.chars().fold(true, Whitespace::is) {
                    return Ok(Token::Whitespace);
                }
                incoming.next()
            } else {
                return Ok(Token::Whitespace);
            };

            loop {
                match incoming.peek().map(|r| r.clone()) {
                    Some((grapheme, _)) => {
                        if grapheme.chars().fold(true, Whitespace::is) {
                            incoming.next();
                        } else {
                            break;
                        }
                    }
                    None => break,
                }
            }
            Ok(Token::Whitespace)
        }
    }
    #[test]
    fn test_lexer() {
        let input: Vec<u8> = "\"My name is\"\n \"Noah Scott\" \" HIIII\"\n  \"Shanaberger\""
            .as_bytes()
            .to_vec()
            .into_iter()
            .chain([0xADu8; 100])
            .collect();
        let input = Cursor::new(input);

        let mut lexer = Lexer::new(input, true, true, Some(Token::Eof))
            .tokenizer(|| DoubleQuotedStringLexer::new())
            .tokenizer(|| Whitespace);

        if let Err(error) = lexer.tokenize() {
            panic!("{}", error)
        }

        println!("Displaying the lexed tokens:");

        for token in lexer.tokens() {
            println!("Next token:");
            println!("{}", textwrap::indent(&format!("{}", token), "\t"));
            if let Token::Eof = token.token() {
                println!("\tString Value: None");
                break;
            }
            println!(
                "\tString Value: {:?}",
                String::from_utf8_lossy(&lexer.bytes()[token.span().byte_range().clone().unwrap()])
            );
        }

        println!("Invalid byte count: {}", lexer.invalid_bytes());
        use super::span::Sourceable;
        println!("Source: {}", lexer.source_string())
    }

    #[test]
    fn test_spans() {
        let input = "\"Hi My Name is Noah\"\n\n      \t\n\"My Name is Noah!\"";

        let input = Cursor::new(input);

        let mut lexer = Lexer::new(input, true, true, Some(Token::Eof))
            .tokenizer(|| DoubleQuotedStringLexer::new())
            .tokenizer(|| Whitespace);

        if let Err(error) = lexer.tokenize() {
            panic!("{}", error)
        }

        println!("Displaying the lexed tokens:");

        let input = String::from_utf8(lexer.bytes().to_vec()).unwrap();

        println!("Input Len: {};", input.len());

        for token in lexer.tokens() {
            println!("Token Value: {:?}", token.token());
            println!("Token Range: {:?}", token.span().byte_range());
            if let Token::Eof = token.token() {
                break;
            }
            println!(
                "Token Bytes (converted to string): {:?}",
                String::from_utf8(
                    (&input[token.span().byte_range().clone().unwrap()])
                        .as_bytes()
                        .to_vec()
                )
                .unwrap()
            )
        }

        println!("Invalid byte count: {}", lexer.invalid_bytes());
        use super::span::Sourceable;
        println!("Source: {}", lexer.source_string())
    }
}
