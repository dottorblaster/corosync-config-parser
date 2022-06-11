pub mod config;
pub mod error;
pub mod lexer;
pub mod parser;

pub use config::ConfigBlock;
pub use error::{Error as ParseError, Result};

pub fn parse(data: String) -> Result<ConfigBlock> {
    let owned_string = Box::leak(data.into_boxed_str());

    match lexer::run(Box::new(owned_string.chars())) {
        Ok(tokens) => parser::run(Box::new(tokens.into_iter())),
        Err(err) => Err(err),
    }
}
