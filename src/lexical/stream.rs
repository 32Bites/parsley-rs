use std::{
    collections::VecDeque,
    io::Read,
    iter::FusedIterator,
    mem,
    ops::{Bound, RangeBounds, RangeInclusive},
};

use character_stream::{CharacterIterator, CharacterStream, CharacterStreamError};
use unicode_segmentation::UnicodeSegmentation;

use super::{error::LexError, SourceableReader};

#[derive(Debug)]
pub struct Blackhole(Vec<u8>, bool, usize);

impl Blackhole {
    pub fn new(void: bool) -> Self {
        Self(vec![], void, 0)
    }

    pub fn len(&self) -> usize {
        if self.1 {
            self.2
        } else {
            self.0.len()
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn extend(&mut self, i: impl IntoIterator<Item = u8>) {
        if self.1 {
            self.2 += i.into_iter().count();
        } else {
            self.0.extend(i)
        }
    }
}

#[derive(Debug)]
pub struct Chars<Reader: Read> {
    pub(crate) incoming: CharacterIterator<Reader>,
    is_lossy: bool,
    failed_count: usize,
    success_count: usize,
    accumulator: Blackhole,
}

impl<Reader: Read> Chars<Reader> {
    pub fn new(incoming: Reader, is_lossy: bool, void: bool) -> Self {
        Self {
            incoming: CharacterIterator::new(CharacterStream::new(incoming, false)),
            is_lossy,
            failed_count: 0,
            success_count: 0,
            accumulator: Blackhole::new(void),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.accumulator.bytes()
    }

    pub fn byte_count(&self) -> usize {
        self.accumulator.len()
    }

    pub fn valid_bytes(&self) -> usize {
        self.success_count
    }

    pub fn invalid_bytes(&self) -> usize {
        self.failed_count
    }
}

pub type CharsResult = Result<(char, RangeInclusive<usize>), CharacterStreamError>;
pub type PeekedCharsResult<'a> = Result<(char, RangeInclusive<usize>), &'a CharacterStreamError>;

impl<Reader: Read> Iterator for Chars<Reader> {
    type Item = CharsResult;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.accumulator.len();
        Some(match self.incoming.next()? {
            Ok(character) => {
                self.success_count += character.len_utf8();
                self.accumulator.extend(character.to_string().bytes());
                let end = self.accumulator.len().saturating_sub(1);
                Ok((character, start..=end))
            }
            Err(error) => {
                let CharacterStreamError(bytes, _) = &error;
                self.failed_count += bytes.len();

                if self.is_lossy {
                    let c = '\u{FFFD}';
                    self.accumulator.extend(c.to_string().bytes());
                    let end = self.accumulator.len().saturating_sub(1);
                    Ok((c, start..=end))
                } else {
                    Err(error)
                }
            }
        })
    }
}

impl<Reader: Read> FusedIterator for Chars<Reader> {}

#[derive(Debug)]
pub struct Clusters<Reader: Read> {
    pub(crate) chars: Chars<Reader>,
    buffer: String,
    ranges: VecDeque<RangeInclusive<usize>>,
    pending_error: Option<CharacterStreamError>,
}

impl<Reader: Read> Clusters<Reader> {
    pub fn new(chars: Reader, is_lossy: bool, void: bool) -> Self {
        Self {
            chars: Chars::new(chars, is_lossy, void),
            buffer: "".into(),
            ranges: VecDeque::new(),
            pending_error: None,
        }
    }

    pub fn valid_bytes(&self) -> usize {
        self.chars.valid_bytes()
    }

    pub fn invalid_bytes(&self) -> usize {
        self.chars.invalid_bytes()
    }

    pub fn bytes(&self) -> &[u8] {
        self.chars.bytes()
    }

    pub fn byte_count(&self) -> usize {
        self.chars.byte_count()
    }

    fn combine_ranges(&mut self, range: impl RangeBounds<usize>) -> Option<RangeInclusive<usize>> {
        let end = self.ranges.len().saturating_sub(1);
        let valid_start = match range.start_bound() {
            Bound::Included(i) => (*i < end),
            Bound::Excluded(e) => (e - 1 < end),
            Bound::Unbounded => true,
        };
        let valid_end = match range.end_bound() {
            Bound::Included(i) => (*i <= end),
            Bound::Excluded(e) => (e - 1 <= end),
            Bound::Unbounded => true,
        };

        if !valid_start || !valid_end {
            return None;
        }

        let ranges = self
            .ranges
            .drain(range)
            .collect::<Vec<RangeInclusive<usize>>>();

        let first = ranges.first()?;
        let last = ranges.last()?;

        Some((*first.start())..=(*last.end()))
    }
}

impl<Reader: Read> Iterator for Clusters<Reader> {
    type Item = Result<(String, RangeInclusive<usize>), CharacterStreamError>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(error) = self.pending_error.take() {
            return Some(Err(error));
        }
        loop {
            match self.chars.next() {
                Some(Ok((character, byte_range))) => {
                    self.buffer.push(character);
                    self.ranges.push_back(byte_range);
                }
                Some(Err(error)) => {
                    if self.buffer.is_empty() {
                        return Some(Err(error));
                    } else {
                        self.pending_error = Some(error);
                        let range = self.combine_ranges(..).unwrap();
                        return Some(Ok((mem::replace(&mut self.buffer, "".into()), range)));
                    }
                }
                None => {
                    if self.buffer.is_empty() {
                        return None;
                    } else {
                        let range = self.combine_ranges(..).unwrap();
                        return Some(Ok((mem::replace(&mut self.buffer, "".into()), range)));
                    }
                }
            }

            let mut gi = self.buffer.grapheme_indices(true).fuse();
            if let (Some((_, first_grapheme)), Some((second_pos, _))) = (gi.next(), gi.next()) {
                let grapheme = first_grapheme.to_owned();
                self.buffer = unsafe { self.buffer.get_unchecked(second_pos..) }.to_owned();
                let range = self
                    .combine_ranges(..(self.ranges.len().saturating_sub(1)))
                    .unwrap();
                return Some(Ok((grapheme, range)));
            }
        }
    }
}

impl<Reader: Read> FusedIterator for Clusters<Reader> {}

#[derive(Clone, Debug)]
pub struct GraphemeLocation {
    pub index: usize,
    pub byte_range: RangeInclusive<usize>,
    pub line: usize,
    pub column: usize,
}

/// A wrapper struct to simplify the utilization of the enumerated multipeek grapheme iterator
/// that is utilized for lexing.
#[derive(Debug)]
pub struct Graphemes<'a> {
    pub(crate) iter: Clusters<Box<dyn SourceableReader + 'a>>,
    count: usize,
    column: usize,
    lines: Vec<String>,
    queue: VecDeque<Result<(String, RangeInclusive<usize>), CharacterStreamError>>,
    peek_index: usize,
    last_byte_index: usize,
}

impl<'a> Graphemes<'a> {
    pub fn new<Reader: SourceableReader + 'a>(reader: Reader, is_lossy: bool, void: bool) -> Self {
        Self {
            iter: Clusters::new(Box::new(reader), is_lossy, void),
            count: 0,
            column: 0,
            lines: vec!["".into()],
            queue: VecDeque::new(),
            peek_index: 0,
            last_byte_index: 0,
        }
    }

    pub fn from<Reader: SourceableReader + 'a>(reader: Reader) -> Self {
        Self::new(reader, true, true)
    }

    pub fn inner(&self) -> &Clusters<Box<dyn SourceableReader + 'a>> {
        &self.iter
    }

    pub fn inner_mut(&mut self) -> &mut Clusters<Box<dyn SourceableReader + 'a>> {
        &mut self.iter
    }

    pub fn bytes(&self) -> &[u8] {
        self.iter.bytes()
    }

    pub fn byte_count(&self) -> usize {
        self.iter.byte_count()
    }

    pub fn valid_bytes(&self) -> usize {
        self.iter.valid_bytes()
    }

    pub fn invalid_bytes(&self) -> usize {
        self.iter.invalid_bytes()
    }

    pub fn grapheme_count(&self) -> usize {
        self.count
    }

    pub fn current_line(&self) -> usize {
        self.lines.len().saturating_sub(1)
    }

    pub fn current_column(&self) -> usize {
        self.column
    }

    pub fn current_index(&self) -> usize {
        self.count.saturating_sub(1)
    }

    pub fn current_byte_index(&self) -> usize {
        self.last_byte_index
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut [String] {
        &mut self.lines
    }

    pub fn peek(&mut self) -> Option<(String, GraphemeLocation)> {
        let result = if self.peek_index < self.queue.len() {
            &self.queue[self.peek_index]
        } else {
            match self.iter.next() {
                Some(x) => {
                    self.queue.push_back(x);
                    &self.queue[self.peek_index]
                }
                None => return None,
            }
        };

        let result: Option<(String, GraphemeLocation)> = match result {
            Ok((grapheme, range)) => {
                let (line, column) = match grapheme.as_str() {
                    "\r\n" | "\n" => (self.lines.len(), 0),
                    _ => (
                        self.lines.len().saturating_sub(1),
                        if self.count == 0 { 0 } else { self.column + 1 },
                    ),
                };

                let location = GraphemeLocation {
                    index: self.current_index(),
                    byte_range: range.clone(),
                    line,
                    column,
                };

                self.peek_index += 1;

                Some((grapheme.clone(), location))
            }
            Err(_) => None,
        };

        result
    }

    pub fn reset_peek(&mut self) {
        self.peek_index = 0;
    }
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = Result<(String, GraphemeLocation), LexError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reset_peek();
        match self.queue.pop_front().or_else(|| self.iter.next()) {
            Some(Ok((grapheme, range))) => {
                self.count += 1;
                if matches!(grapheme.as_str(), "\r\n" | "\n") {
                    self.column = 0;
                    self.lines.push("".into())
                } else {
                    if let Some(last) = self.lines.last_mut() {
                        if self.count != 1 {
                            self.column += 1;
                        }
                        last.push_str(&grapheme);
                    }
                }
                self.last_byte_index = *range.end();

                let location = GraphemeLocation {
                    index: self.current_index(),
                    byte_range: range,
                    line: self.current_line(),
                    column: self.current_column(),
                };

                Some(Ok((grapheme, location)))
            }
            Some(Err(error)) => Some(Err(LexError::other(error))),
            None => None,
        }
    }
}

impl<'a> FusedIterator for Graphemes<'a> {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_graphemes() {
        let input = "Hello, My name is \n \r\n \r\n\r \n\t\r\nNoah!";
        let cursor = std::io::Cursor::new(input);

        let mut graphemes = super::Graphemes::new(cursor, false, false);

        loop {
            match graphemes.peek() {
                Some(p) => {
                    println!("Peek: {:?}", p);
                    let next = graphemes
                        .next()
                        .and_then(|s| s.ok())
                        .map_or("None".to_string(), |s| format!("{s:?}"));
                    println!("Next: {}", next);
                }
                None => {
                    graphemes.reset_peek();
                    break;
                }
            }
        }
    }
}
