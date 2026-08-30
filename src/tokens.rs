use tiktoken_rs::cl100k_base;

#[derive(Debug, Clone, Copy)]
pub struct TokenStats {
    pub original_bytes: usize,
    pub reduced_bytes: usize,
    pub original_tokens: usize,
    pub reduced_tokens: usize,
}

impl TokenStats {
    pub fn compute(original: &str, reduced: &str) -> Self {
        let bpe = cl100k_base().ok();

        let original_tokens = match &bpe {
            Some(enc) => enc.encode_with_special_tokens(original).len(),
            None => original.len() / 4,
        };

        let reduced_tokens = match &bpe {
            Some(enc) => enc.encode_with_special_tokens(reduced).len(),
            None => reduced.len() / 4,
        };

        Self {
            original_bytes: original.len(),
            reduced_bytes: reduced.len(),
            original_tokens,
            reduced_tokens,
        }
    }

    pub fn savings_percent(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        ((self.original_tokens.saturating_sub(self.reduced_tokens)) as f64
            / self.original_tokens as f64)
            * 100.0
    }

    pub fn format_summary(&self) -> String {
        format!(
            "Tokens: {} -> {} ({:.1}% saved) | Bytes: {} -> {} ({:.1}% saved)",
            self.original_tokens,
            self.reduced_tokens,
            self.savings_percent(),
            self.original_bytes,
            self.reduced_bytes,
            if self.original_bytes > 0 {
                ((self.original_bytes.saturating_sub(self.reduced_bytes)) as f64
                    / self.original_bytes as f64)
                    * 100.0
            } else {
                0.0
            }
        )
    }
}
