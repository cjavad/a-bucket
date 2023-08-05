use std::str::Split;

pub trait TextStream {
    fn as_line_stream<'a>(data: &'a str) -> Split<'a, &str>;
    fn as_token_stream<'a>(line: &'a str) -> Split<'a, &str>;
}
