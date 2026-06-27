//! N-gram language model loader for ARPA disambiguation.
//!
//! Reads a standard ARPA LM file (as emitted by `lmplz`, `ngram-train`, or
//! any other Kneser-Ney trainer) and answers: "given the previous char,
//! which of the candidate target characters is most likely?"
//!
//! The ARPA format is text:
//!   \\data\\
//!   ngram 1=N
//!   ngram 2=M
//!   ngram 3=K
//!   \\1-grams:
//!   -2.34  word
//!   -1.23  word     backoff
//!   \\2-grams:
//!   -3.45  w1 w2
//!   -2.67  w1 w2   backoff
//!   ...
//!   \\end\\
//!
//! We load 1-gram, 2-gram and 3-gram tables into HashMaps keyed by their
//! n-gram. The disambig query uses the 2-gram (and 3-gram when
//! available) to score each candidate and returns the highest-probability
//! one. If a candidate has no entry in the model, it scores
//! `-f64::INFINITY` (worst), and falls back to the input order (so the
//! existing "first value" behavior is preserved).
use std::collections::HashMap;
use std::fs;
use std::io;

/// An ARPA-format n-gram language model. Stores log10 probabilities.
#[derive(Debug, Default)]
pub struct NgramModel {
    pub(crate) unigrams: HashMap<String, f64>,
    pub(crate) bigrams: HashMap<(String, String), f64>,
    pub(crate) trigrams: HashMap<(String, String, String), f64>,
}

impl NgramModel {
    /// Parse an ARPA LM file.
    pub fn from_file(path: &str) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        Self::parse(&text)
    }

    /// Parse ARPA LM text.
    pub fn parse(text: &str) -> io::Result<Self> {
        let mut model = NgramModel::default();
        let mut section: &str = "";
        for raw_line in text.lines() {
            let line = raw_line.trim_end();
            if line.starts_with('\\') {
                section = line;
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let toks: Vec<&str> = line.split_whitespace().collect();
            // <unk> / <s> / </s> usually appear with logprob=0, backoff=0.
            // Format: logprob word [backoff]   (1-gram)
            //         logprob w1 w2 [backoff]   (2-gram)
            //         logprob w1 w2 w3          (3-gram)
            if toks.len() < 2 {
                continue;
            }
            let logp: f64 = toks[0].parse().unwrap_or(f64::NEG_INFINITY);
            match (section, toks.len()) {
                ("\\1-grams:", 2) | ("\\1-grams:", 3) => {
                    model.unigrams.insert(toks[1].to_string(), logp);
                }
                ("\\2-grams:", 3) | ("\\2-grams:", 4) => {
                    model
                        .bigrams
                        .insert((toks[1].to_string(), toks[2].to_string()), logp);
                }
                ("\\3-grams:", 4) => {
                    model.trigrams.insert(
                        (
                            toks[1].to_string(),
                            toks[2].to_string(),
                            toks[3].to_string(),
                        ),
                        logp,
                    );
                }
                _ => {}
            }
        }
        Ok(model)
    }

    /// Pick the best candidate given the previous character context.
    /// `prev` is the previous char in the source text (or None for the
    /// start of a sentence). Returns the candidate with the highest
    /// log10 bigram probability. On ties, the first candidate wins.
    /// If a candidate is missing from the model, it scores -inf; if all
    /// candidates are missing, the first one is returned as the fallback.
    pub fn disambiguate(
        &self,
        prev: Option<&str>,
        candidates: &[String],
    ) -> Option<String> {
        if candidates.is_empty() {
            return None;
        }
        if candidates.len() == 1 {
            return Some(candidates[0].clone());
        }
        // Map a char (e.g. "<s>") to a stable first char if prev is None.
        let prev_key = prev.unwrap_or("<s>").to_string();
        let mut best_idx = 0;
        let mut best_score = self
            .bigrams
            .get(&(prev_key.clone(), candidates[0].clone()))
            .copied()
            .unwrap_or(f64::NEG_INFINITY);
        for (i, cand) in candidates.iter().enumerate().skip(1) {
            let s = self
                .bigrams
                .get(&(prev_key.clone(), cand.clone()))
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            if s > best_score {
                best_idx = i;
                best_score = s;
            }
        }
        Some(candidates[best_idx].clone())
    }

    /// Number of loaded bigrams (for stats / debugging).
    pub fn bigram_count(&self) -> usize {
        self.bigrams.len()
    }

    /// Raw access to the unigram table, for cloning (CLI builds one
    /// `Converter` per region).
    pub fn unigrams_raw(&self) -> &HashMap<String, f64> {
        &self.unigrams
    }

    /// Raw access to the bigram table, for cloning.
    pub fn bigrams_raw(&self) -> &HashMap<(String, String), f64> {
        &self.bigrams
    }

    /// Raw access to the trigram table, for cloning.
    pub fn trigrams_raw(&self) -> &HashMap<(String, String, String), f64> {
        &self.trigrams
    }

    /// Cheap-ish deep clone (ARPA models are a few MB).
    pub fn clone_model(&self) -> NgramModel {
        NgramModel {
            unigrams: self.unigrams.clone(),
            bigrams: self.bigrams.clone(),
            trigrams: self.trigrams.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_model() -> NgramModel {
        let arpa = r#"\data\
ngram 1=3
ngram 2=4

\1-grams:
-1.0  齣
-1.5  出
-1.0  好

\2-grams:
-2.0  齣 好
-3.0  出 好
-2.5  齣 齣
-2.0  齣 出

\end\"#;
        NgramModel::parse(arpa).expect("parse")
    }

    #[test]
    fn parses_unigrams_and_bigrams() {
        let m = make_test_model();
        assert!(m.bigrams.contains_key(&("齣".to_string(), "好".to_string())));
        assert!(m.bigrams.contains_key(&("齣".to_string(), "齣".to_string())));
    }

    #[test]
    fn disambiguate_picks_highest_prob() {
        let m = make_test_model();
        // log P(齣|齣) = -2.5; log P(出|齣) = -2.0; 出 wins
        let pick = m.disambiguate(Some("齣"), &["齣".to_string(), "出".to_string()]);
        assert_eq!(pick, Some("出".to_string()));
    }

    #[test]
    fn disambiguate_falls_back_to_first_when_all_missing() {
        let m = NgramModel::default();
        // Empty model — every candidate is missing, fall back to first
        let pick = m.disambiguate(Some("X"), &["齣".to_string(), "出".to_string()]);
        assert_eq!(pick, Some("齣".to_string()));
    }
}
