#[derive(Debug)]
/// Represents an error that occurs when lexing a character.
pub enum LexError {
    Character(String),
    Other(String),
    StartNewToken(bool),
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = match self {
            Self::Character(error) | Self::Other(error) => error.to_owned(),
            Self::StartNewToken(should_reuse_character) => {
                let string = match should_reuse_character {
                    true => "",
                    false => " not",
                };
                format!("The provided character is invalid, so create a new token, and you should{} reuse the current character.", string)
            }
        };

        write!(f, "{}", error)
    }
}

impl std::error::Error for LexError {}
