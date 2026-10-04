//! This library provides rust implementations for some stemmer algorithms
//! written in the [snowball language](https://snowballstem.org/).
//!
//!
//! All algorithms expect the input to already be lowercased.
//!
//! # Usage
//! ```toml
//! [dependencies]
//! rust-stemmers = "^1.0"
//! ```
//!
//! ```rust
//! extern crate rust_stemmers;
//!
//! use rust_stemmers::{Algorithm, Stemmer};
//!
//! fn main() {
//!    let en_stemmer = Stemmer::create(Algorithm::English);
//!    assert_eq!(en_stemmer.stem("fruitlessly"), "fruitless");
//! }
//! ```
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::borrow::Cow;

mod snowball;

use snowball::SnowballEnv;
use snowball::algorithms;

/// Enum of all supported algorithms.
/// Check the [Snowball-Website](https://snowballstem.org/) for details.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Copy, Clone)]
pub enum Algorithm {
    Arabic,
    Danish,
    Dutch,
    English,
    Finnish,
    French,
    German,
    Greek,
    Hungarian,
    Italian,
    Norwegian,
    Portuguese,
    Romanian,
    Russian,
    Spanish,
    Swedish,
    Tamil,
    Turkish
}

/// Wrapps a usable interface around the actual stemmer implementation
pub struct Stemmer {
    stemmer: fn(&mut SnowballEnv) -> bool,
}

impl Stemmer {
    /// Create a new stemmer from an algorithm
    pub fn create(lang: Algorithm) -> Self {
        match lang {
            Algorithm::Arabic => Stemmer { stemmer: algorithms::arabic::stem },
            Algorithm::Danish => Stemmer { stemmer: algorithms::danish::stem },
            Algorithm::Dutch => Stemmer { stemmer: algorithms::dutch::stem },
            Algorithm::English => Stemmer { stemmer: algorithms::english::stem },
            Algorithm::Finnish => Stemmer { stemmer: algorithms::finnish::stem },
            Algorithm::French => Stemmer { stemmer: algorithms::french::stem },
            Algorithm::German => Stemmer { stemmer: algorithms::german::stem },
            Algorithm::Greek => Stemmer { stemmer: algorithms::greek::stem },
            Algorithm::Hungarian => Stemmer { stemmer: algorithms::hungarian::stem },
            Algorithm::Italian => Stemmer { stemmer: algorithms::italian::stem },
            Algorithm::Norwegian => Stemmer { stemmer: algorithms::norwegian::stem },
            Algorithm::Portuguese => Stemmer { stemmer: algorithms::portuguese::stem },
            Algorithm::Romanian => Stemmer { stemmer: algorithms::romanian::stem },
            Algorithm::Russian => Stemmer { stemmer: algorithms::russian::stem },
            Algorithm::Spanish => Stemmer { stemmer: algorithms::spanish::stem },
            Algorithm::Swedish => Stemmer { stemmer: algorithms::swedish::stem },
            Algorithm::Tamil => Stemmer { stemmer: algorithms::tamil::stem },
            Algorithm::Turkish => Stemmer { stemmer: algorithms::turkish::stem },
        }
    }

    /// Stem a single word
    /// Please note, that the input is expected to be all lowercase (if that is applicable).
    pub fn stem<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut env = SnowballEnv::create(input);
        (self.stemmer)(&mut env);
        env.get_current()
    }
}



#[cfg(test)]
mod tests {
    use super::{Stemmer, Algorithm};

    fn stemms_to(lhs: &str, rhs: &str, stemmer: Algorithm) {
        assert_eq!(Stemmer::create(stemmer).stem(lhs), rhs);
    }

    #[test]
    fn german_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_ger.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_ger.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::German);
        }
    }

    #[test]
    fn english_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_en.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_en.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::English);
        }
    }

    #[test]
    fn french_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_fr.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_fr.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::French);
        }
    }

    #[test]
    fn spanish_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_es.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_es.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Spanish);
        }
    }

    #[test]
    fn portuguese_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_pt.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_pt.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Portuguese);
        }
    }

    #[test]
    fn italian_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_it.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_it.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Italian);
        }
    }

    #[test]
    fn romanian_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_ro.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_ro.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Romanian);
        }
    }

    #[test]
    fn russian_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_ru.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_ru.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Russian);
        }
    }

    #[test]
    fn arabic_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_ar.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_ar.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Arabic);
        }
    }

    #[test]
    fn finnish_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_fi.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_fi.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Finnish);
        }
    }

    #[test]
    fn greek_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_el.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_el.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Greek);
        }
    }

    #[test]
    fn norwegian_test() {
        use std::fs;
        use std::io;
        use std::io::BufRead;

        let vocab = io::BufReader::new(fs::File::open("test_data/voc_no.txt").unwrap());
        let result = io::BufReader::new(fs::File::open("test_data/res_no.txt").unwrap());

        let lines = vocab.lines().zip(result.lines());

        for (voc, res) in lines {
            stemms_to(voc.unwrap().as_str(),
                      res.unwrap().as_str(),
                      Algorithm::Norwegian);
        }
    }

}
