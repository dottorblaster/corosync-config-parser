pub mod config;
pub mod error;
pub mod lexer;
pub mod parser;

pub use config::ConfigBlock;
pub use error::{Error as ParseError, Result};

pub fn parse(data: String) -> Result<ConfigBlock> {
    let chars = Box::leak(data.into_boxed_str());
    let corosync_tokens = lexer::run(Box::new(chars.chars())).unwrap();
    println!("{:?}", corosync_tokens);

    let corosync_conf = parser::run(Box::new(corosync_tokens.into_iter()));
    println!("{:?}", corosync_conf);
    corosync_conf
}
