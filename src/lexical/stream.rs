use std::{
    cell::RefCell,
    io::{Error, ErrorKind, Read, Result as IoResult},
    rc::Rc,
};

use character_stream::{CharacterIterator, CharacterStream, CharacterStreamError};
use itertools::{Itertools, MultiPeek};

/// Wrapper for [character_stream::CharacterIterator] that ensures compatibility with [unicode_reader::Graphemes].
pub struct Chars<Reader: Read> {
    incoming: CharacterIterator<Reader>,
    is_lossy: bool,
    failed_count: Option<Rc<RefCell<usize>>>,
}

impl<Reader: Read> Chars<Reader> {
    /// Create a [Chars] from `reader`.
    /// `is_lossy` determines whether the stream will replace invalid UTF-8 byte sequences with a U+FFFD.
    ///
    /// If `is_lossy` is false, then [Chars::next] will return an Error containing a [character_stream::CharacterStreamError], which
    /// provides the bytes that failed to be recognized as valid UTF-8, in addition to the error that resulted from the failed parsing.
    pub fn new(reader: Reader, is_lossy: bool, failed_count: Option<Rc<RefCell<usize>>>) -> Self {
        Self {
            incoming: CharacterIterator::new(CharacterStream::new(reader, false)),
            failed_count,
            is_lossy,
        }
    }

    /// Returns the amount of invalid UTF-8 bytes.
    pub fn invalid(&self) -> usize {
        self.failed_count.as_ref().map_or(0, |c| *c.borrow())
    }
}

impl<Reader: Read> Iterator for Chars<Reader> {
    type Item = IoResult<char>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.incoming.next()? {
            Ok(character) => Ok(character),
            Err(error) => {
                let CharacterStreamError(bytes, boxed_error) = error;
                if let Some(ref count) = self.failed_count {
                    *count.borrow_mut() += bytes.len();
                }
                if self.is_lossy {
                    Ok('\u{FFFD}')
                } else {
                    match boxed_error.downcast::<Error>() {
                        Ok(error) => Err(*error),
                        Err(error) => Err(Error::new(
                            ErrorKind::Other,
                            CharacterStreamError(bytes, error),
                        )),
                    }
                }
            }
        })
    }
}

impl<Reader: Read> From<CharacterIterator<Reader>> for Chars<Reader> {
    fn from(iter: CharacterIterator<Reader>) -> Self {
        let is_lossy = iter.is_lossy();
        Self {
            incoming: iter,
            failed_count: None,
            is_lossy,
        }
    }
}

impl<Reader: Read> From<CharacterStream<Reader>> for Chars<Reader> {
    fn from(stream: CharacterStream<Reader>) -> Self {
        let is_lossy = stream.is_lossy;
        Self {
            incoming: CharacterIterator::new(stream),
            failed_count: None,
            is_lossy,
        }
    }
}

#[derive(Clone, Debug)]
/// Describes where a grapheme is from the start of the input.
pub struct GraphemeLocation {
    /// The index of the grapheme, barring invalid UTF-8 sequences.
    pub index: usize,
    /// Which line the grapheme is on, starting at zero.
    pub line: usize,
    /// The offset from the start of the line in which the grapheme lies.
    pub offset: usize,
}

impl GraphemeLocation {
    pub fn new(index: usize, line: usize, offset: usize) -> Self {
        Self {
            index,
            line,
            offset,
        }
    }
}

/// A wrapper struct to simplify the utilization of the enumerated multipeek grapheme iterator
/// that is utilized for lexing.
pub struct Graphemes {
    iter: MultiPeek<unicode_reader::Graphemes<Chars<Box<dyn Read>>>>,
    successful_reads: usize,
    failed_reads: usize,
    line: usize,
    line_offset: usize,
    invalid_bytes: Rc<RefCell<usize>>,
}

impl Graphemes {
    pub fn new<Reader: Read + 'static>(reader: Reader, is_lossy: bool) -> Self {
        let invalid_bytes = Rc::new(RefCell::new(0));
        Self {
            iter: unicode_reader::Graphemes::from(Chars::new(
                Box::new(reader) as Box<dyn Read>,
                is_lossy,
                Some(invalid_bytes.clone()),
            ))
            .multipeek(),
            successful_reads: 0,
            failed_reads: 0,
            line: 0,
            line_offset: 0,
            invalid_bytes: invalid_bytes.clone(),
        }
    }

    pub fn from<Reader: Read + 'static>(reader: Reader) -> Self {
        Self::new(reader, true)
    }

    pub fn peek(&mut self) -> Option<(usize, &IoResult<String>)> {
        let index = self.current_index() + 1;
        match self.iter.peek() {
            Some(result) => Some((index, result)),
            None => None,
        }
    }

    pub fn reset_peek(&mut self) {
        self.iter.reset_peek()
    }

    pub fn inner(&self) -> &MultiPeek<unicode_reader::Graphemes<Chars<Box<dyn Read>>>> {
        &self.iter
    }

    pub fn inner_mut(&mut self) -> &mut MultiPeek<unicode_reader::Graphemes<Chars<Box<dyn Read>>>> {
        &mut self.iter
    }

    pub fn successes(&self) -> usize {
        self.successful_reads
    }

    pub fn failures(&self) -> usize {
        self.failed_reads
    }

    pub fn attempts_total(&self) -> usize {
        self.successful_reads + self.failed_reads
    }

    pub fn current_index(&self) -> usize {
        self.successful_reads.saturating_sub(1)
    }

    pub fn lines(&self) -> usize {
        self.line + 1
    }

    pub fn invalid_bytes(&self) -> usize {
        *self.invalid_bytes.borrow()
    }
}

impl Iterator for Graphemes {
    type Item = Result<(GraphemeLocation, String), (usize, Error)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(Ok(grapheme)) => {
                if grapheme == "\n" {
                    self.line += 1;
                    self.line_offset = 0;
                } else {
                    self.line_offset += 1;
                }
                self.successful_reads += 1;
                let location =
                    GraphemeLocation::new(self.current_index(), self.line, self.line_offset);
                Some(Ok((location, grapheme)))
            }
            Some(Err(error)) => {
                self.failed_reads += 1;
                Some(Err((self.current_index() + 1, error)))
            }
            None => None,
        }
    }
}
