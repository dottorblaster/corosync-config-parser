use super::config::ConfigBlock;
use super::error::{CodePosition, Error, ErrorType, Result};
use super::lexer;
use super::lexer::{Token, TokenType};

macro_rules! expect_token {
    ($state:expr) => {
        match next($state) {
            Some(t) => t,
            None => return fail($state, ErrorType::UnexpectedEOF, "token")
        }
    };
    ($state:expr, $ty:expr) => {
        match next($state) {
            Some(Token {token_type: $ty(..), ..} @ t) => t,
            Some(t) => return fail($state, ErrorType::Unexpected(t), stringify!($ty))
            None => return fail($state, ErrorType::UnexpectedEOF, stringify!($ty))
        }
    }
}

struct ParseState {
    tokens: Box<dyn Iterator<Item = lexer::Token>>,
    last_token: Option<Token>,
    force_next: Option<Token>,
    done: bool,
}

impl CodePosition for ParseState {
    fn location(&self) -> (u32, u16) {
        match self.last_token {
            Some(ref t) => (t.line, t.col),
            None => (0, 0),
        }
    }
}

pub fn run(tokens: Box<dyn Iterator<Item = lexer::Token>>) -> Result<ConfigBlock> {
    let mut state = ParseState {
        tokens: tokens,
        last_token: None,
        force_next: None,
        done: false,
    };

    parse_block(&mut state, false, String::from(""), vec![])
}

fn parse_block(
    state: &mut ParseState,
    inner: bool,
    name: String,
    options: Vec<String>,
) -> Result<ConfigBlock> {
    let mut return_value = ConfigBlock::new(name, options, vec![]);
    loop {
        let token = if inner {
            expect_token!(state)
        } else {
            match next(state) {
                Some(t) => t,
                None => return Ok(return_value),
            }
        };
        match token.clone().token_type {
            TokenType::RawLiteral(option_name) => {
                let params = parse_params(state)?;
                let t = expect_token!(state);
                match t.token_type {
                    TokenType::OpenBrace => {
                        // Block follows
                        return_value.add_block(parse_block(state, true, option_name, params)?);
                    }
                    _ => {
                        // No block. In strict mode this will only ever execute for
                        // TokenType::Semicolon as parse_params() will already have
                        // returned an error for other types
                        return_value.add_block(ConfigBlock::new(option_name, params, vec![]))
                    }
                }
            }
            TokenType::CloseBrace if inner => break,
            TokenType::Semicolon => {}
            _ => {
                return fail(
                    state,
                    ErrorType::Unexpected(token.clone()),
                    if inner { "option or }" } else { "option" },
                )
            }
        }
    }
    Ok(return_value)
}

fn parse_params(state: &mut ParseState) -> Result<Vec<String>> {
    let mut return_value = vec![];
    loop {
        match lookahead(state) {
            Some(t) => match t.token_type {
                TokenType::StringLiteral(s) => {
                    return_value.push(s);
                    pop(state);
                }
                TokenType::RawLiteral(s) => {
                    return_value.push(s);
                    pop(state);
                }
                TokenType::OpenBrace => break,
                TokenType::Semicolon => break,
                TokenType::LineEnd => break,
                TokenType::Colon => {
                    pop(state);
                }
                _ => {
                    if cfg!(feature = "nonstrict") {
                        break;
                    } else {
                        println!("Errored params");
                        return fail(state, ErrorType::Unexpected(t), "; or {");
                    }
                }
            },
            None => return fail(&state, ErrorType::UnexpectedEOF, "}"),
        }
    }
    Ok(return_value)
}

fn next(state: &mut ParseState) -> Option<lexer::Token> {
    let v = match &state.force_next {
        &Some(ref t) => Some(t.clone()),
        &None => state.tokens.next(),
    };
    state.force_next = None;
    match v.clone() {
        Some(_) => {}
        None => {
            if state.done {
                unreachable!("Tried to get another token after end of stream");
            } else {
                state.done = true;
            }
        }
    }
    v
}

fn pop(state: &mut ParseState) {
    if state.force_next.is_some() {
        state.force_next = None;
    } else {
        state.tokens.next();
    }
}

fn lookahead(state: &mut ParseState) -> Option<lexer::Token> {
    match state.force_next.clone() {
        Some(t) => Some(t),
        None => {
            let t = state.tokens.next();
            state.force_next = t.clone();
            t
        }
    }
}

fn fail<T>(state: &ParseState, error_type: ErrorType, expected: &'static str) -> Result<T> {
    Err(Error::from_state(state, error_type, Some(expected)))
}

#[cfg(test)]
mod test {
    use super::super::config::ConfigBlock;
    use super::super::lexer::{Token, TokenType};
    use super::*;

    #[test]
    fn test_it_parsing_the_most_basic_option() {
        assert_eq!(
            run(Box::new(
                vec![
                    tok(TokenType::RawLiteral(String::from("test"))),
                    tok(TokenType::Semicolon)
                ]
                .into_iter()
            )),
            Ok(ConfigBlock::new(
                String::new(),
                vec![],
                vec![ConfigBlock::new(String::from("test"), vec![], vec![])]
            ))
        );
    }

    #[test]
    fn test_it_parsing_a_typical_example() {
        assert_eq!(
            run(Box::new(
                vec![
                    tok(TokenType::RawLiteral(String::from("option"))),
                    tok(TokenType::RawLiteral(String::from("param1"))),
                    tok(TokenType::OpenBrace),
                    tok(TokenType::RawLiteral(String::from("inner"))),
                    tok(TokenType::StringLiteral(String::from("value"))),
                    tok(TokenType::Semicolon),
                    tok(TokenType::CloseBrace),
                ]
                .into_iter()
            )),
            Ok(ConfigBlock::new(
                String::new(),
                vec![],
                vec![ConfigBlock::new(
                    String::from("option"),
                    vec![String::from("param1")],
                    vec![ConfigBlock::new(
                        String::from("inner"),
                        vec![String::from("value")],
                        vec![]
                    )]
                )]
            ))
        );
    }

    fn tok(ty: TokenType) -> Token {
        Token::new(0, 0, ty)
    }
}
