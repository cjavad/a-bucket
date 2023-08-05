use std::collections::HashMap;

use crate::http::{self, MimeType};

pub struct Response {
    required_authentication: bool,

    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status_code: u16) -> Self {
        Self {
            required_authentication: false,
            status_code,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn mark_required_authentication(&mut self) {
        self.required_authentication = true;
    }

    pub fn set_status_code(&mut self, status_code: u16) {
        self.status_code = status_code;
    }

    pub fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    pub fn set_cookie(&mut self, key: &str, value: &str, httponly: bool) {
        self.headers
            .insert("set-cookie".to_string(), format!("{}={}{}", key, value, if httponly { "; HttpOnly" } else { "" }));
    }

    pub fn set_body(&mut self, body: Vec<u8>, mime_type: MimeType) {
        self.set_header("content-length", &body.len().to_string());
        self.set_header("content-type", mime_type.to_str());
        self.body = body
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status_code,
            http::status_code_lookup(self.status_code)
        );


        for (key, value) in &self.headers {
            if key == "set-cookie" && !self.required_authentication {
                continue;
            }

            response.push_str(&format!("{}: {}\r\n", key, value));
        }

        response.push_str("\r\n");

        let mut response_bytes: Vec<u8> = response.into_bytes();

        response_bytes.extend(&self.body);

        response_bytes
    }
}
