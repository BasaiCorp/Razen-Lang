use std::fs;
use std::path::Path;
use crate::token::{Token, TokenType, lookup_identifier};

#[derive(Clone)]
pub struct Lexer {
    input: String,
    position: usize,      // current position in input (points to current char)
    read_position: usize, // current reading position in input (after current char)
    ch: char,             // current char under examination
    line: usize,          // current line number
    column: usize,        // current column number
}

impl Lexer {
    pub fn new(input: String) -> Self {
        let mut lexer = Lexer {
            input,
            position: 0,
            read_position: 0,
            ch: '\0',
            line: 1,
            column: 0,
        };
        lexer.read_char();
        lexer
    }
    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(content) => Ok(Lexer::new(content)),
            Err(e) => Err(format!("Could not read file: {}", e)),
        }
    }
    
    fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.ch = '\0';
        } else {
            self.ch = self.input.chars().nth(self.read_position).unwrap();
        }
        self.position = self.read_position;
        self.read_position += 1;
        self.column += 1;
    }
    
    fn peek_char(&self) -> char {
        if self.read_position >= self.input.len() {
            '\0'
        } else {
            self.input.chars().nth(self.read_position).unwrap()
        }
    }
    
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        
        let token = match self.ch {
            '=' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::Equal, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Assign, self.ch.to_string(), self.line, self.column)
                }
            },
            '+' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::PlusAssign, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Plus, self.ch.to_string(), self.line, self.column)
                }
            },
            '-' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::MinusAssign, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Minus, self.ch.to_string(), self.line, self.column)
                }
            },
            '!' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::NotEqual, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Not, self.ch.to_string(), self.line, self.column)
                }
            },
            '/' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::SlashAssign, literal, self.line, self.column - 1)
                } else if self.peek_char() == '/' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::FloorDiv, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Slash, self.ch.to_string(), self.line, self.column)
                }
            },
            '*' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::AsteriskAssign, literal, self.line, self.column - 1)
                } else if self.peek_char() == '*' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::Power, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Asterisk, self.ch.to_string(), self.line, self.column)
                }
            },
            '%' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::PercentAssign, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Percent, self.ch.to_string(), self.line, self.column)
                }
            },
            '<' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::LessEqual, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Less, self.ch.to_string(), self.line, self.column)
                }
            },
            '>' => {
                if self.peek_char() == '=' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::GreaterEqual, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Greater, self.ch.to_string(), self.line, self.column)
                }
            },
            '&' => {
                if self.peek_char() == '&' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::And, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Illegal, self.ch.to_string(), self.line, self.column)
                }
            },
            '|' => {
                if self.peek_char() == '|' {
                    let ch = self.ch;
                    self.read_char();
                    let literal = format!("{}{}", ch, self.ch);
                    Token::new(TokenType::Or, literal, self.line, self.column - 1)
                } else {
                    Token::new(TokenType::Illegal, self.ch.to_string(), self.line, self.column)
                }
            },
            ';' => Token::new(TokenType::Semicolon, self.ch.to_string(), self.line, self.column),
            ':' => Token::new(TokenType::Colon, self.ch.to_string(), self.line, self.column),
            '(' => Token::new(TokenType::LeftParen, self.ch.to_string(), self.line, self.column),
            ')' => Token::new(TokenType::RightParen, self.ch.to_string(), self.line, self.column),
            ',' => Token::new(TokenType::Comma, self.ch.to_string(), self.line, self.column),
            // This case is already handled above
            // '+' => Token::new(TokenType::Plus, self.ch.to_string(), self.line, self.column),
            '{' => Token::new(TokenType::LeftBrace, self.ch.to_string(), self.line, self.column),
            '}' => Token::new(TokenType::RightBrace, self.ch.to_string(), self.line, self.column),
            '[' => Token::new(TokenType::LeftBracket, self.ch.to_string(), self.line, self.column),
            ']' => Token::new(TokenType::RightBracket, self.ch.to_string(), self.line, self.column),
            '.' => Token::new(TokenType::Dot, self.ch.to_string(), self.line, self.column),
            '#' => {
                let comment = self.read_comment();
                let len = comment.len();
                Token::new(TokenType::Comment, comment, self.line, self.column - len)
            },
            '"' => {
                let string = self.read_string();
                let len = string.len();
                Token::new(TokenType::StringLiteral, string, self.line, self.column - len)
            },
            '\0' => Token::new(TokenType::EOF, "".to_string(), self.line, self.column),
            _ => {
                if is_letter(self.ch) {
                    let literal = self.read_identifier();
                    let token_type = lookup_identifier(&literal);
                    return Token::new(token_type, literal.clone(), self.line, self.column - literal.len());
                } else if is_digit(self.ch) {
                    let literal = self.read_number();
                    return Token::new(TokenType::NumberLiteral, literal.clone(), self.line, self.column - literal.len());
                } else {
                    Token::new(TokenType::Illegal, self.ch.to_string(), self.line, self.column)
                }
            }
        };
        
        self.read_char();
        token
    }
    
    fn skip_whitespace(&mut self) {
        while self.ch.is_whitespace() {
            if self.ch == '\n' {
                self.line += 1;
                self.column = 0;
            }
            self.read_char();
        }
    }
    
    fn read_identifier(&mut self) -> String {
        let position = self.position;
        while is_letter(self.ch) || is_digit(self.ch) {
            self.read_char();
        }
        self.input[position..self.position].to_string()
    }
    
    fn read_number(&mut self) -> String {
        let position = self.position;
        let mut has_dot = false;
        
        while is_digit(self.ch) || (self.ch == '.' && !has_dot) {
            if self.ch == '.' {
                has_dot = true;
            }
            self.read_char();
        }
        
        self.input[position..self.position].to_string()
    }
    
    fn read_string(&mut self) -> String {
        // Skip the opening quote
        self.read_char();
        
        let position = self.position;
        while self.ch != '"' && self.ch != '\0' {
            if self.ch == '\n' {
                self.line += 1;
                self.column = 0;
            }
            self.read_char();
        }
        
        let string = self.input[position..self.position].to_string();
        string
    }
    
    fn read_comment(&mut self) -> String {
        // Skip the # character
        self.read_char();
        
        let position = self.position;
        while self.ch != '\n' && self.ch != '\0' {
            self.read_char();
        }
        
        self.input[position..self.position].to_string()
    }
    
    pub fn tokenize_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token();
            let is_eof = token.token_type == TokenType::EOF;
            
            tokens.push(token);
            
            if is_eof {
                break;
            }
        }
        
        tokens
    }
}

fn is_letter(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_digit(ch: char) -> bool {
    ch.is_digit(10)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_next_token() {
        let input = r#"let x = 5;
        take greeting = "hello";
        if (x > 10) {
            show x;
        } else {
            show greeting;
        }
        # This is a comment
        "#;
        
        let mut lexer = Lexer::new(input.to_string());
        
        let expected_tokens = vec![
            (TokenType::Let, "let"),
            (TokenType::Identifier, "x"),
            (TokenType::Assign, "="),
            (TokenType::NumberLiteral, "5"),
            (TokenType::Semicolon, ";"),
            (TokenType::Take, "take"),
            (TokenType::Identifier, "greeting"),
            (TokenType::Assign, "="),
            (TokenType::StringLiteral, "hello"),
            (TokenType::Semicolon, ";"),
            (TokenType::If, "if"),
            (TokenType::LeftParen, "("),
            (TokenType::Identifier, "x"),
            (TokenType::Greater, ">"),
            (TokenType::NumberLiteral, "10"),
            (TokenType::RightParen, ")"),
            (TokenType::LeftBrace, "{"),
            (TokenType::Show, "show"),
            (TokenType::Identifier, "x"),
            (TokenType::Semicolon, ";"),
            (TokenType::RightBrace, "}"),
            (TokenType::Else, "else"),
            (TokenType::LeftBrace, "{"),
            (TokenType::Show, "show"),
            (TokenType::Identifier, "greeting"),
            (TokenType::Semicolon, ";"),
            (TokenType::RightBrace, "}"),
            (TokenType::Comment, " This is a comment"),
            (TokenType::EOF, ""),
        ];
        
        for (expected_type, expected_literal) in expected_tokens {
            let token = lexer.next_token();
            assert_eq!(token.token_type, expected_type, "Expected token type {:?}, got {:?}", expected_type, token.token_type);
            assert_eq!(token.literal, expected_literal, "Expected token literal '{}', got '{}'", expected_literal, token.literal);
        }
    }
}