#[derive(Debug, PartialEq, Eq)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
    PATCH,
    LIST
}

impl Method {
    pub fn from_str(token: &str) -> Result<Self, String> {
        match token {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "CONNECT" => Ok(Method::CONNECT),
            "TRACE" => Ok(Method::TRACE),
            "PATCH" => Ok(Method::PATCH),
            "LIST" => Ok(Method::LIST),
            _ => Err("Invalid method".into()),
        }
    }
}

#[derive(Default)]
pub enum MimeType {
    TextPlain,
    TextHtml,
    TextCss,
    TextJavascript,
    ImagePng,
    ImageJpeg,
    ImageGif,
    ImageWebp,
    ImageSvg,
    ApplicationJson,
    ApplicationXml,
    #[default]
    ApplicationOctetStream,
}

impl MimeType {
    pub fn from_str(token: &str) -> Self {
        match token {
            "text/plain" => MimeType::TextPlain,
            "text/html" => MimeType::TextHtml,
            "text/css" => MimeType::TextCss,
            "text/javascript" => MimeType::TextJavascript,
            "image/png" => MimeType::ImagePng,
            "image/jpeg" => MimeType::ImageJpeg,
            "image/gif" => MimeType::ImageGif,
            "image/webp" => MimeType::ImageWebp,
            "image/svg+xml" => MimeType::ImageSvg,
            "application/json" => MimeType::ApplicationJson,
            "application/xml" => MimeType::ApplicationXml,
            "application/octet-stream" => MimeType::ApplicationOctetStream,
            _ => MimeType::ApplicationOctetStream,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            MimeType::TextPlain => "text/plain",
            MimeType::TextHtml => "text/html",
            MimeType::TextCss => "text/css",
            MimeType::TextJavascript => "text/javascript",
            MimeType::ImagePng => "image/png",
            MimeType::ImageJpeg => "image/jpeg",
            MimeType::ImageGif => "image/gif",
            MimeType::ImageWebp => "image/webp",
            MimeType::ImageSvg => "image/svg+xml",
            MimeType::ApplicationJson => "application/json",
            MimeType::ApplicationXml => "application/xml",
            MimeType::ApplicationOctetStream => "application/octet-stream",
        }
    }

    pub fn is_utf8(&self) -> bool {
        match self {
            MimeType::TextPlain => true,
            MimeType::TextHtml => true,
            MimeType::TextCss => true,
            MimeType::TextJavascript => true,
            MimeType::ApplicationJson => true,
            _ => false,
        }
    }
}

pub fn status_code_lookup(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",

        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",

        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",

        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        418 => "I'm a teapot",

        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",

        _ => "Unknown",
    }
}
