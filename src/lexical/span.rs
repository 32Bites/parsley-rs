use character_stream::{CharacterIterator, CharacterStream};
use std::{
    any,
    fmt::{Debug, Display},
    fs::File,
    io::{Cursor, Read},
    net::TcpStream,
    ops::{Deref, DerefMut, RangeInclusive},
};

use super::{Chars, Clusters, Graphemes, Lexer, TokenValue};

/// Trait for representing a lexical source
pub trait Sourceable {
    fn source_string(&self) -> String;
}

/// Trait for representing a reader that is sourceable.
pub trait SourceableReader: Sourceable + Read + Debug {}

impl<T: Read + Sourceable + Debug> SourceableReader for T {}

impl<Reader: SourceableReader> Sourceable for CharacterStream<Reader> {
    fn source_string(&self) -> String {
        self.as_ref().source_string()
    }
}

impl<Reader: SourceableReader> Sourceable for CharacterIterator<Reader> {
    fn source_string(&self) -> String {
        self.stream().source_string()
    }
}

impl<Reader: SourceableReader> Sourceable for Chars<Reader> {
    fn source_string(&self) -> String {
        self.incoming.source_string()
    }
}

impl<'a, R: AsRef<dyn SourceableReader + 'a>> Sourceable for R {
    fn source_string(&self) -> String {
        self.as_ref().source_string()
    }
}

impl<Reader: SourceableReader> Sourceable for Clusters<Reader> {
    fn source_string(&self) -> String {
        self.chars.source_string()
    }
}

impl Sourceable for Graphemes<'_> {
    fn source_string(&self) -> String {
        self.iter.source_string()
    }
}

impl<TokenType: TokenValue> Sourceable for Lexer<'_, TokenType> {
    fn source_string(&self) -> String {
        self.incoming.source_string()
    }
}

#[cfg(feature = "buffer")]
impl<B: AsRef<[u8]> + any::Any> Sourceable for Cursor<B> {
    fn source_string(&self) -> String {
        let any_bytes = self.get_ref() as &dyn any::Any;

        match (
            any_bytes.downcast_ref::<String>(),
            any_bytes.downcast_ref::<&str>(),
        ) {
            (Some(s), None) => format!("Cursor over String at {:p}", s),
            (None, Some(s)) => format!("Cursor over &str at {:p}", s),
            (None, None) => format!(
                "Cursor over {} at {:p}",
                any::type_name::<B>(),
                self.get_ref()
            ),
            (Some(_), Some(_)) => unreachable!(),
        }
    }
}

#[cfg(feature = "net")]
impl Sourceable for TcpStream {
    fn source_string(&self) -> String {
        match self.peer_addr() {
            Ok(addr) => {
                let host = {
                    #[cfg(feature = "dns")]
                    {
                        use dns_lookup::lookup_addr;
                        match lookup_addr(&addr.ip()) {
                            Ok(domain) => domain,
                            Err(_) => addr.ip().to_string(),
                        }
                    }
                    #[cfg(not(feature = "dns"))]
                    {
                        addr.ip().to_string()
                    }
                };

                format!("tcp://{}:{}", host, addr.port())
            }
            Err(_) => "Closed TCP Connection".into(),
        }
    }
}

#[cfg(feature = "fs")]
impl Sourceable for File {
    fn source_string(&self) -> String {
        parsley_rs_hack::source_string(self)
    }
}

/// Trait for types that implement [Read], [Display] and [Debug].
pub trait DisplayableReader: Read + Display + Debug {}
impl<T: Read + Display + Debug> DisplayableReader for T {}

/// Trait for types that implement [DisplayableReader] to wrap it in a type that
/// implements [SourceableReader].
pub trait ToSource: DisplayableReader + Sized {
    fn to_source(self) -> Source<Self>;
}

impl<DR: DisplayableReader + Sized> ToSource for DR {
    fn to_source(self) -> Source<Self> {
        Source::from(self)
    }
}

/// Trait for types that implement [Read] and [Debug].
pub trait DebugableReader: Read + Debug {}
impl<T: Read + Debug> DebugableReader for T {}

/// Trait for types that implement [DebugableReader] to wrap it in a type that
/// implements [SourceableReader].
pub trait ToDebugSource: DebugableReader + Sized {
    fn to_debug_source(self, pretty_print: bool) -> DebugSource<Self>;
    fn to_debug_source_pretty(self) -> DebugSource<Self>;
    fn to_debug_source_unpretty(self) -> DebugSource<Self>;
}

impl<DR: DebugableReader + Sized> ToDebugSource for DR {
    fn to_debug_source(self, pretty_print: bool) -> DebugSource<Self> {
        DebugSource::from(self, pretty_print)
    }

    fn to_debug_source_pretty(self) -> DebugSource<Self> {
        DebugSource::pretty_from(self)
    }

    fn to_debug_source_unpretty(self) -> DebugSource<Self> {
        DebugSource::unpretty_from(self)
    }
}

#[derive(Debug)]
/// Type that implements [Read] and [Sourceable] for its wrapped value that implements [DisplayableReader].
pub struct Source<DR: DisplayableReader>(pub DR);

impl<DR: DisplayableReader> Display for Source<DR> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source_string())
    }
}

impl<DR: DisplayableReader> Source<DR> {
    pub fn from(reader: DR) -> Self {
        Self(reader)
    }
}

impl<DR: DisplayableReader> Sourceable for Source<DR> {
    fn source_string(&self) -> String {
        self.0.to_string()
    }
}

impl<DR: DisplayableReader> Read for Source<DR> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl<DR: DisplayableReader> Deref for Source<DR> {
    type Target = DR;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<DR: DisplayableReader> DerefMut for Source<DR> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<DR: DisplayableReader> AsRef<DR> for Source<DR> {
    fn as_ref(&self) -> &DR {
        &self.0
    }
}

impl<DR: DisplayableReader> AsMut<DR> for Source<DR> {
    fn as_mut(&mut self) -> &mut DR {
        &mut self.0
    }
}

#[derive(Debug)]
/// Type that implements [Sourceable] and [Read] for a [DebugableReader] stored as a parameter.
pub struct DebugSource<DR: DebugableReader>(pub DR, pub bool);

impl<DR: DebugableReader> DebugSource<DR> {
    pub fn from(reader: DR, pretty_print: bool) -> Self {
        Self(reader, pretty_print)
    }

    pub fn pretty_from(reader: DR) -> Self {
        Self::from(reader, true)
    }

    pub fn unpretty_from(reader: DR) -> Self {
        Self::from(reader, false)
    }
}

impl<DR: DebugableReader> Read for DebugSource<DR> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl<DR: DebugableReader> Sourceable for DebugSource<DR> {
    fn source_string(&self) -> String {
        match self.1 {
            true => format!("{:#?}", self.0),
            false => format!("{:?}", self.0),
        }
    }
}

impl<DR: DebugableReader> Deref for DebugSource<DR> {
    type Target = DR;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<DR: DebugableReader> DerefMut for DebugSource<DR> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<DR: DebugableReader> AsRef<DR> for DebugSource<DR> {
    fn as_ref(&self) -> &DR {
        &self.0
    }
}

impl<DR: DebugableReader> AsMut<DR> for DebugSource<DR> {
    fn as_mut(&mut self) -> &mut DR {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
/// Type that represents where on a line a given item is located.
pub struct Line {
    /// The line number, starting from zero.
    pub index: usize,
    /// Grapheme range on line.
    pub range: RangeInclusive<usize>,
}

fn generate_lines(
    lines: &[String],
    line_range: RangeInclusive<usize>,
    column_range: RangeInclusive<usize>,
) -> Vec<Line> {
    let mut out_lines: Vec<Line> = vec![];

    if *line_range.start() == *line_range.end() {
        out_lines.push(Line {
            index: *line_range.start(),
            range: column_range,
        })
    } else {
        for l in line_range {
            let s = "".to_string();
            let max = Clusters::new(Cursor::new(lines.get(l).unwrap_or(&s)), true, true).count(); //.saturating_sub(1);

            out_lines.push(Line {
                index: l,
                range: 0..=max,
            })
        }

        let last = out_lines.last_mut().unwrap();

        last.range = 0..=(*column_range.end());
    }

    out_lines
}

/// Represents where in the stream a token lies.
#[derive(Debug, Default, Clone)]
pub struct Span {
    /// The lines the token lies on.
    pub lines: Vec<Line>,
    /// The range of graphemes where the token is located.
    pub grapheme_range: Option<RangeInclusive<usize>>,
    /// The range of bytes where the token is located.
    pub byte_range: Option<RangeInclusive<usize>>,
    /// The source from which the token was read.
    pub source: String,
}

impl Span {
    pub fn new(
        lines: &[String],
        grapheme_range: RangeInclusive<usize>,
        byte_range: RangeInclusive<usize>,
        line_range: RangeInclusive<usize>,
        column_range: RangeInclusive<usize>,
        source: &impl Sourceable,
    ) -> Self {
        let lines = generate_lines(lines, line_range, column_range);

        Self {
            lines,
            grapheme_range: Some(grapheme_range),
            byte_range: Some(byte_range),
            source: source.source_string(),
        }
    }

    pub fn source(&self) -> &str {
        match self.source.is_empty() {
            false => &self.source,
            true => "No Source",
        }
    }

    pub fn lines(&self) -> &Vec<Line> {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut Vec<Line> {
        &mut self.lines
    }

    pub fn grapheme_range(&self) -> &Option<RangeInclusive<usize>> {
        &self.grapheme_range
    }

    pub fn grapheme_range_mut(&mut self) -> &mut Option<RangeInclusive<usize>> {
        &mut self.grapheme_range
    }

    pub fn byte_range(&self) -> &Option<RangeInclusive<usize>> {
        &self.byte_range
    }

    pub fn byte_range_mut(&mut self) -> &mut Option<RangeInclusive<usize>> {
        &mut self.byte_range
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lines = match self.lines.len() {
            0 => "Lines: None".into(),
            1 => {
                let line = &self.lines[0];
                format!(
                    "Line: [number: {}, columns: <start: {}, end: {}> ]",
                    line.index,
                    line.range.start(),
                    line.range.end()
                )
            }
            _ => {
                let first_line = self.lines.first().unwrap();
                let last_line = self.lines.last().unwrap();
                format!(
                    "Lines: [start: {}, end: {}, end_columns: <start: 0, end: {}>]",
                    first_line.index,
                    last_line.index,
                    last_line.range.end()
                )
            }
        };
        writeln!(f, "Source: {}", self.source())?;
        writeln!(
            f,
            "\tGrapheme Range: {}",
            self.grapheme_range
                .as_ref()
                .map(|g| format!("{:?}", g))
                .unwrap_or("Empty".into())
        )?;
        writeln!(
            f,
            "\tByte Range: {}",
            self.byte_range
                .as_ref()
                .map(|b| format!("{:?}", b))
                .unwrap_or("Empty".into())
        )?;
        writeln!(f, "\t{}", lines)
    }
}
