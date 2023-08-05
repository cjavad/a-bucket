// HTTP Request parser
// Builds up a request from a stream of bytes

use std::str::Split;

use crate::{http::Method, request::Request, text_stream::TextStream};

const CRLF_CHARS: &str = "\r\n";
const CRLF_BYTES: &[u8] = b"\r\n";
const TOKEN_SEPERATOR: &str = " ";
const HEADER_SEPERATOR: &str = ": ";

pub struct Parser {
    buffer: Vec<u8>,

    has_parsed_request_line: bool,
    has_consumed_req_headers: bool,

    is_invalid: bool,

    request: Option<Request>,
}

// Default parsing behavior for HTTP
impl TextStream for Parser {
    fn as_line_stream<'a>(data: &'a str) -> Split<'a, &str> {
        data.split(CRLF_CHARS)
    }

    fn as_token_stream<'a>(line: &'a str) -> Split<'a, &str> {
        line.split(TOKEN_SEPERATOR)
    }
}

impl Parser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            has_parsed_request_line: false,
            has_consumed_req_headers: false,
            is_invalid: false,
            request: None,
        }
    }

    pub fn consume_request(self) -> Option<Request> {
        self.request
    }

    pub fn update(&mut self, raw_data: &[u8]) {
        if self.is_invalid {
            return;
        }

        self.buffer.extend(raw_data);

        while let Some(line) = self.consume_until_body() {
            if !self.has_parsed_request_line {
                self.parse_request_line(line);
            } else {
                self.parse_request_header(line);
            }
        }

        if self.has_consumed_req_headers {
            self.parse_request_body();
        }
    }

    fn consume_until_body(&mut self) -> Option<String> {
        if self.has_consumed_req_headers {
            return None;
        }

        // Check if buffer contains \r\n, then parse (and consume) the line
        if let Some(index) = self
            .buffer
            .windows(CRLF_BYTES.len())
            .position(|x| x == CRLF_BYTES)
        {
            // If the next window also contains \r\n, then we have a blank line and we have parsed all headers
            if self.buffer[index + CRLF_BYTES.len()..].starts_with(CRLF_BYTES) {
                self.has_consumed_req_headers = true;
            }

            // Consume the line
            let drained = self.buffer.drain(..index + CRLF_BYTES.len());
            let line = String::from_utf8_lossy(&drained.as_slice());
            let line = line.as_ref();
            let line = &line[..line.len() - CRLF_CHARS.len()];

            Some(line.to_string())
        } else {
            None
        }
    }

    fn parse_request_line(&mut self, line: String) {
        // Parse the line
        let mut tokens = Self::as_token_stream(line.as_str());

        let method = tokens.next();
        let uri = tokens.next();
        let version = tokens.next();

        if method.is_none() || uri.is_none() || version.is_none() {
            self.is_invalid = true;
            return;
        }

        if !version.unwrap().starts_with("HTTP/") {
            self.is_invalid = true;
            return;
        }

        let method = match Method::from_str(method.unwrap()) {
            Ok(method) => method,
            Err(_) => {
                self.is_invalid = true;
                return;
            }
        };

        let uri = uri.unwrap().to_string();
        let version: String = version.unwrap().to_string();

        self.request = Some(Request::new(method, uri, version));
        self.has_parsed_request_line = true;
    }

    fn parse_request_header(&mut self, line: String) {
        let (header_name, header_value) = match line.split_once(HEADER_SEPERATOR) {
            Some((header_name, header_value)) => (header_name, header_value),
            None => {
                self.is_invalid = true;
                return;
            }
        };

        let header_name = header_name.trim().to_lowercase();
        let header_value = header_value.to_string();

        if let Some(request) = &mut self.request {
            if request.headers.contains_key(header_name.as_str()) {
                request.headers.remove(header_name.as_str());
            }

            request.headers.insert(header_name, header_value);
        }

        if self.has_consumed_req_headers {
            // Ensure content length and such are parsed as the last header
            self.request.as_mut().unwrap().post_process();
            // Drain last CRLF
            self.buffer.drain(..CRLF_BYTES.len());
        }
    }

    fn parse_request_body(&mut self) {
        // Append rest of buffer to body
        if let Some(request) = &mut self.request {
            request.raw_body.extend(self.buffer.drain(..));
        }
    }

    pub fn is_done(&self) -> bool {
        if self.is_invalid() {
            return true;
        }

        if !self.has_consumed_req_headers {
            return false;
        }

        if let Some(request) = &self.request {
            if let Some(content_length) = request.content_length {
                return request.raw_body.len() >= content_length;
            }
        }

        return true;
    }

    pub fn is_invalid(&self) -> bool {
        self.is_invalid || self.request.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update() {
        let mut parser = Parser::new();

        let data = b"GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\n";
        parser.update(data);

        assert!(!parser.is_invalid());
        assert!(parser.is_done());

        let request = parser.request.as_ref().unwrap();

        assert_eq!(request.method, Method::GET);
        assert_eq!(request.uri, "/");
        assert_eq!(request.version, "HTTP/1.1");

        assert_eq!(
            request.headers.get("host"),
            Some(&"example.com".to_string())
        );
        assert_eq!(
            request.headers.get("content-length"),
            Some(&"0".to_string())
        );
    }

    #[test]
    fn test_update_with_invalid_data() {
        let mut parser = Parser::new();

        let data = b"INVALID DATA";
        parser.update(data);

        assert!(parser.is_invalid());
        assert!(parser.is_done());
    }
}
