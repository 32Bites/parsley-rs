use std::io::{BufRead, Error, ErrorKind, Result, Seek};

use character_stream::{CharacterIterator, CharacterStream, CharacterStreamError};

/// Wrapper for [character_stream::CharacterIterator] that ensures compatibility with [unicode_reader::Graphemes].
pub struct Chars<Reader: BufRead + Seek> {
    incoming: CharacterIterator<Reader>,
}

impl<Reader: BufRead + Seek> Chars<Reader> {
    pub fn new(reader: Reader, is_lossy: bool) -> Self {
        Self {
            incoming: CharacterIterator::new(CharacterStream::new(reader, is_lossy)),
        }
    }

    pub fn from(reader: Reader) -> Self {
        Self::new(reader, true)
    }
}

impl<Reader: BufRead + Seek> Iterator for Chars<Reader> {
    type Item = Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.incoming.next()? {
            Ok(character) => Ok(character),
            Err(error) => {
                let CharacterStreamError(bytes, boxed_error) = error;
                match boxed_error.downcast::<Error>() {
                    Ok(error) => Err(*error),
                    Err(error) => Err(Error::new(
                        ErrorKind::Other,
                        CharacterStreamError(bytes, error),
                    )),
                }
            }
        })
    }
}

impl<Reader: BufRead + Seek> From<CharacterIterator<Reader>> for Chars<Reader> {
    fn from(iter: CharacterIterator<Reader>) -> Self {
        Self { incoming: iter }
    }
}

impl<Reader: BufRead + Seek> From<CharacterStream<Reader>> for Chars<Reader> {
    fn from(stream: CharacterStream<Reader>) -> Self {
        Self {
            incoming: CharacterIterator::new(stream),
        }
    }
}
