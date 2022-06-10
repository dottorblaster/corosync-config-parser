use super::error::{CodePosition, Error, ErrorType, Result};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum LexerMode {
    None,
    String,
    Raw,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenType {
    StringLiteral(String),
    RawLiteral(String),
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    Semicolon,
    Colon,
    LineEnd,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: u32,
    pub col: u16,
}

impl Token {
    pub fn new(line: u32, col: u16, ty: TokenType) -> Token {
        Token {
            line: line,
            col: col,
            token_type: ty,
        }
    }
}

struct LexerState {
    line: u32,
    col: u16,
    input: Box<dyn Iterator<Item = char>>,
    mode: LexerMode,
    escaped: bool,
    tmp: String,
    tokens: Vec<Token>,
    force_next: Option<char>,
}

impl CodePosition for LexerState {
    fn location(&self) -> (u32, u16) {
        (self.line, self.col)
    }
}

fn end_token(state: &mut LexerState) {
    if state.mode != LexerMode::None {
        let t = match state.mode {
            LexerMode::None => unreachable!("Invalid mode when generating token"),
            LexerMode::String => Token::new(
                state.line,
                state.col,
                TokenType::StringLiteral(state.tmp.clone()),
            ),
            LexerMode::Raw => Token::new(
                state.line,
                state.col,
                TokenType::RawLiteral(state.tmp.clone()),
            ),
        };
        state.mode = LexerMode::None;
        state.tokens.push(t);
    }
}

fn start_token(state: &mut LexerState, mode: LexerMode) {
    if state.mode != LexerMode::None {
        end_token(state);
    }
    state.tmp = String::new();
    state.mode = mode;
}

fn append_token(state: &mut LexerState, t: TokenType) {
    end_token(state);
    state.tokens.push(Token::new(state.line, state.col, t));
}

pub fn run(input: Box<dyn Iterator<Item = char>>) -> Result<Vec<Token>> {
    let mut state = LexerState {
        line: 1,
        col: 0,
        input: input,
        mode: LexerMode::None,
        escaped: false,
        tmp: String::new(),
        tokens: vec![],
        force_next: None,
    };
    loop {
        let c = { next(&mut state) };
        let mode = state.mode.clone();
        let esc = state.escaped;
        match (c, mode, esc) {
            (Some('"'), LexerMode::String, false) => {
                end_token(&mut state);
            }
            (Some('"'), LexerMode::None, false) => {
                start_token(&mut state, LexerMode::String);
            }
            (Some('\\'), LexerMode::String, false) => {
                state.escaped = true;
            }
            (Some('\\'), LexerMode::String, true) => {
                state.tmp.push('\\');
                state.escaped = false;
            }
            (Some('n'), LexerMode::String, true) => {
                state.tmp.push('\n');
                state.escaped = false;
            }
            (Some(x), LexerMode::String, false) => {
                state.tmp.push(x);
            }
            (None, LexerMode::String, _) => return fail(&state, ErrorType::UnexpectedEOF),
            (Some(' '), LexerMode::None, false) => {}
            (Some(' '), LexerMode::Raw, false) => {
                end_token(&mut state);
            }
            (Some('('), LexerMode::Raw, false) => {
                append_token(&mut state, TokenType::OpenParen);
            }
            (Some(')'), LexerMode::Raw, false) => {
                append_token(&mut state, TokenType::CloseParen);
            }
            (Some('{'), LexerMode::Raw, false) => {
                append_token(&mut state, TokenType::OpenBrace);
            }
            (Some('}'), LexerMode::Raw, false) => {
                append_token(&mut state, TokenType::CloseBrace);
            }
            (Some(':'), LexerMode::Raw, false) => {
                append_token(&mut state, TokenType::Colon);
            }
            (Some('('), LexerMode::None, false) => {
                append_token(&mut state, TokenType::OpenParen);
            }
            (Some(')'), LexerMode::None, false) => {
                append_token(&mut state, TokenType::CloseParen);
            }
            (Some('{'), LexerMode::None, false) => {
                append_token(&mut state, TokenType::OpenBrace);
            }
            (Some('}'), LexerMode::None, false) => {
                append_token(&mut state, TokenType::CloseBrace);
            }
            (Some(';'), LexerMode::None, false) => {
                append_token(&mut state, TokenType::Semicolon);
            }
            (Some(':'), LexerMode::None, false) => {
                append_token(&mut state, TokenType::Colon);
            }
            (Some('\n'), LexerMode::None, false) => {}
            (Some('\n'), LexerMode::Raw, false) => append_token(&mut state, TokenType::LineEnd),
            (Some(x), LexerMode::None, false) => {
                start_token(&mut state, LexerMode::Raw);
                state.tmp.push(x);
            }
            (Some(x), LexerMode::Raw, false) => {
                state.tmp.push(x);
            }
            (None, LexerMode::Raw, false) => {
                end_token(&mut state);
                break;
            }
            (None, LexerMode::None, false) => {
                break;
            }
            (c, mode, esc) => {
                unreachable!(
                    "Invalid Parser State Reached: {:?}, {:?}, {:?}",
                    c, mode, esc
                );
            }
        }
    }
    Ok(state.tokens)
}

fn fail<T>(state: &LexerState, error_type: ErrorType) -> Result<T> {
    Err(Error::from_state(state, error_type, None))
}

#[derive(Debug, Clone, Copy)]
enum PreProcessorState {
    Default,
    LineComment,
    MultiComment(u8),
}

fn next_char(state: &mut LexerState) -> Option<char> {
    match state.force_next {
        Some(c) => {
            state.force_next = None;
            Some(c)
        }
        None => state.input.next(),
    }
}

fn lookahead(state: &mut LexerState) -> Option<char> {
    match state.force_next {
        Some(c) => Some(c),
        None => {
            let c = state.input.next();
            state.force_next = c;
            c
        }
    }
}

fn next(state: &mut LexerState) -> Option<char> {
    let mut line = state.line;
    let mut column = state.col;
    let mut result: Option<char> = None;
    let mut pre_processor_state = PreProcessorState::Default;
    loop {
        let character = match next_char(state) {
            Some(c) => c,
            None => break,
        };
        match (character, pre_processor_state) {
            ('\n', PreProcessorState::Default) => {
                line += 1;
                column = 0;
                result = Some('\n');
                break;
            }
            ('\r', PreProcessorState::Default) => {}
            ('/', PreProcessorState::Default) => {
                column += 1;
                let n = lookahead(state);
                match n {
                    Some('/') => pre_processor_state = PreProcessorState::LineComment,
                    Some('*') => pre_processor_state = PreProcessorState::MultiComment(1),
                    _ => {
                        result = Some(character);
                        break;
                    }
                }
            }
            ('#', PreProcessorState::Default) => {
                pre_processor_state = PreProcessorState::LineComment;
                column += 1;
            }
            (c, PreProcessorState::Default) if c.is_whitespace() => {
                column += 1;
                result = Some(' ');
                break;
            }
            (_, PreProcessorState::Default) => {
                result = Some(character);
                column += 1;
                break;
            }

            ('\n', PreProcessorState::LineComment) => {
                line += 1;
                column = 0;
                result = Some(' ');
                break;
            }
            (_, PreProcessorState::LineComment) => {
                column += 1;
            }

            ('\n', PreProcessorState::MultiComment(_)) => {
                line += 1;
                column = 0;
            }
            ('*', PreProcessorState::MultiComment(level)) => {
                match lookahead(state) {
                    Some('/') => {
                        if level <= 1 {
                            next(state).unwrap(); // pop the next char
                            pre_processor_state = PreProcessorState::Default
                        } else {
                            pre_processor_state = PreProcessorState::MultiComment(level - 1)
                        }
                    }
                    _ => {}
                }
            }
            ('/', PreProcessorState::MultiComment(level)) => match lookahead(state) {
                Some('*') => {
                    pre_processor_state = PreProcessorState::MultiComment(level + 1);
                }
                _ => {}
            },
            (_, PreProcessorState::MultiComment(_)) => {
                column += 1;
            }
        }
    }
    state.line = line;
    state.col = column;
    result
}

#[cfg(test)]
mod test {
    use super::super::error::{Error, ErrorType, Result};
    use super::*;

    #[test]
    fn successfully_parses_empty_string() {
        assert_eq!(run(Box::new("".chars())), Ok(vec![]));
    }

    #[test]
    fn successfully_parses_raw_token() {
        assert_eq!(
            unwrap_tokens(run(Box::new("test".chars()))),
            Ok(vec![TokenType::RawLiteral(String::from("test"))])
        );
    }

    #[test]
    fn successfully_parses_string_token() {
        assert_eq!(
            unwrap_tokens(run(Box::new("\"test\"".chars()))),
            Ok(vec![TokenType::StringLiteral(String::from("test"))])
        );
    }

    #[test]
    fn successfully_parse_basic_tokens() {
        assert_eq!(
            unwrap_tokens(run(Box::new("(){};:".chars()))),
            Ok(vec![
                TokenType::OpenParen,
                TokenType::CloseParen,
                TokenType::OpenBrace,
                TokenType::CloseBrace,
                TokenType::Semicolon,
                TokenType::Colon,
            ])
        );
    }

    fn unwrap_tokens(tokens: Result<Vec<Token>>) -> Result<Vec<TokenType>> {
        tokens.map(|toks| toks.iter().map(|t| t.token_type.clone()).collect())
    }

    #[test]
    fn successfully_parse_a_typical_example() {
        assert_eq!(
            unwrap_tokens(run(Box::new(
                "option param { inner_option \"value\"; };".chars()
            ))),
            Ok(vec![
                TokenType::RawLiteral(String::from("option")),
                TokenType::RawLiteral(String::from("param")),
                TokenType::OpenBrace,
                TokenType::RawLiteral(String::from("inner_option")),
                TokenType::StringLiteral(String::from("value")),
                TokenType::Semicolon,
                TokenType::CloseBrace,
                TokenType::Semicolon
            ])
        );
    }

    #[test]
    fn ignores_comments() {
        assert_eq!(
            unwrap_tokens(run(Box::new(
                "/* shit */
                                       // crap
                                       # shit"
                    .chars()
            ))),
            Ok(vec![])
        );
    }

    #[test]
    fn fails_on_unterminated_string() {
        assert_eq!(
            run(Box::new("\"yo dawg".chars())),
            Err(Error::new(1, 8, ErrorType::UnexpectedEOF, None))
        );
    }
}
