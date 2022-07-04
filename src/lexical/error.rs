#[derive(Debug)]
/// Represents an error that occurs when lexing a character.
pub enum LexError {
    Character(String),
    Other(String),
    StartNewToken(bool),
    CharacterRead(Vec<u8>, Box<dyn std::error::Error>),
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
            Self::CharacterRead(bytes, error) => {
                format!("Character read error on bytes {:?}: {}", bytes, error)
            }
        };

        write!(f, "{}", error)
    }
}

impl std::error::Error for LexError {}
