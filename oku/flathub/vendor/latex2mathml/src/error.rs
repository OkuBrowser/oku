use std::fmt;
use super::token::Token;

#[derive(Debug, Clone)]
pub enum LatexError {
    UnexpectedToken {
        expected: Token, got: Token,
    },
    MissingParensethis {
        location: Token, got: Token,
    },
    UnknownEnvironment(String),
    InvalidNumberOfDollarSigns,
}

impl fmt::Display for LatexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatexError::UnexpectedToken{expected, got} => write!(f, 
                "The token \"{:?}\" is expected, but the token \"{:?}\" is found.\"", 
                expected, got
            ),
            LatexError::MissingParensethis{location, got} => write!(f, 
                "There must be a parenthesis after \"{:?}\", but not found. Insted, \"{:?}\" is found.",
                location, got
            ),
            LatexError::UnknownEnvironment(environment) => write!(f,
                "An unknown environment \"{}\" is found", environment
            ),
            LatexError::InvalidNumberOfDollarSigns => write!(f,
                "The number of dollar sings found is invalid."
            ),
        }
    }
}

impl std::error::Error for LatexError {}
