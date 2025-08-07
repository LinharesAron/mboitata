use hyper::HeaderMap;
use sanitize_filename::sanitize;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Debug, Clone)]
pub struct InterceptedResponse {
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub content_encoding: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl InterceptedResponse {
    pub fn new(
        path: String,
        scheme: String,
        host: String,
        headers: &HeaderMap,
        body: Vec<u8>,
    ) -> Self {
        let content_type = headers
            .get("content-type")
            .and_then(|val| val.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let content_encoding = headers
            .get("content-encoding")
            .and_then(|val| val.to_str().ok())
            .unwrap_or("identity")
            .to_string();

        InterceptedResponse {
            path,
            scheme,
            host,
            content_encoding,
            content_type,
            body,
        }
    }

    pub fn safe_join(&self, base: &Path) -> Option<PathBuf> {
        let clean_host = sanitize(&self.host);
        
         let clean_components: Vec<String> = Path::new(&self.path)
            .components()
            .filter_map(|comp| comp.as_os_str().to_str())
            .map(|s| sanitize(s))
            .collect();

        let mut final_path = base.join(clean_host);
        for component in clean_components {
            final_path = final_path.join(component);
        }

        if final_path.starts_with(base) {
            Some(final_path)
        } else {
            eprintln!(
                "Tentativa de path traversal detectada! Host: {}, Path: {}",
                self.host, self.path
            );
            None
        }
    }

    pub fn get_url(&self) -> Result<Url, url::ParseError> {
        let full_url = format!("{}://{}{}", self.scheme, self.host, self.path);
        Url::parse(&full_url)
    }

    pub fn get_body(&self) -> Option<String> {
        match self.content_encoding.as_str() {
            "gzip" => {
                let mut gz = flate2::read::GzDecoder::new(self.body.as_slice());
                let mut s = String::new();
                gz.read_to_string(&mut s).ok();
                Some(s)
            }
            "br" => {
                let mut decoder = brotli::Decompressor::new(self.body.as_slice(), 4096);
                let mut s = String::new();
                decoder.read_to_string(&mut s).ok();
                Some(s)
            }
            "identity" | "" => String::from_utf8(self.body.to_vec()).ok(),
            other => {
                println!("⚠️ Encoding não suportado: {}", other);
                None
            }
        }
    }
}
