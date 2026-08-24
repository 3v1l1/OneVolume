#[derive(Debug, Clone)]
pub struct Diagnostics {
    messages: Vec<String>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn add(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    // Not called yet — planned for a "clear diagnostics" UI action
    // once the diagnostics panel does more than just accumulate.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn all(&self) -> &[String] {
        &self.messages
    }
}
