use std::collections::HashMap;

use crate::http::{Method, MimeType};

pub struct Request {
    pub method: Method,
    pub uri: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub raw_body: Vec<u8>,

    // Post-processing
    pub content_length: Option<usize>,
    pub cookies: Option<HashMap<String, String>>,
    pub mime_type: Option<MimeType>,

    // Based on mime type
    pub body: Option<String>,
}

impl Request {
    // Create empty request with no data and default values
    pub fn new(method: Method, uri: String, version: String) -> Self {
        Self {
            method,
            uri,
            version,
            headers: HashMap::new(),
            raw_body: Vec::new(),
            content_length: None,
            cookies: None,
            mime_type: None,
            body: None,
        }
    }

    pub fn post_process(&mut self) {
        self.content_length = self
            .headers
            .get("content-length")
            .map(|x| x.parse::<usize>().unwrap());

        self.cookies = self.headers.get("cookie").map(|x| {
            let mut cookies = HashMap::new();
            for cookie in x.split(";") {
                let Some((key, value)) = cookie.split_once("=") else { continue };
                cookies.insert(key.trim().to_string(), value.to_string());
            }
            cookies
        });

        self.mime_type = self.headers.get("content-type").map(|x| {
            let mut split = x.split(";");
            let mime_type = split.next().unwrap().trim();
            // let charset = split.next().unwrap_or("utf-8");
            MimeType::from_str(mime_type)
        });
    }

    pub fn body_as_string(&mut self) {
        if self.mime_type.as_ref().unwrap().is_utf8() {
            self.body = Some(String::from_utf8_lossy(&self.raw_body).to_string());
        }
    }
}
