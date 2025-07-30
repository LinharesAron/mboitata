#[derive(Clone)]
pub struct AllowList {
    domains: Vec<String>,
}

impl AllowList {
    pub fn new(domains: Vec<String>) -> Self {
        Self { domains }
    }

    pub fn in_scope(&self, url: &str) -> bool {
        if self.domains.is_empty() {
            return true;
        }
        self.domains.iter().any(|domain| url.contains(domain))
    }
}
