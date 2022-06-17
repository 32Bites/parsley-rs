#[derive(Debug)]
/// Represents an error that occurs when lexing a character.
pub enum LexCharacterError {
    StartNewToken { reuse_character: bool },
    OtherError(Box<dyn std::error::Error>),
    Other(String),
}

impl std::fmt::Display for LexCharacterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartNewToken { reuse_character } => {
                let should_reuse = match reuse_character {
                    true => "",
                    false => " not",
                };
                write!(f, "The provided character is invalid, so create a new token, and you should{} reuse the current character.", should_reuse)
            }

            Self::OtherError(error) => write!(f, "{}", error),
            Self::Other(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for LexCharacterError {}
