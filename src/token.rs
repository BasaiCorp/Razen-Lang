use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenType {
    // Variable declaration keywords
    Let,    // Integer, Float, Number
    Take,   // String, Text
    Hold,   // Boolean
    Put,    // Any
    
    // Mathematical operators
    Plus,       // +
    Minus,      // -
    Asterisk,   // *
    Slash,      // /
    Percent,    // %
    Power,      // **
    FloorDiv,   // //
    
    // Assignment operators
    Assign,         // =
    PlusAssign,     // +=
    MinusAssign,    // -=
    AsteriskAssign, // *=
    SlashAssign,    // /=
    PercentAssign,  // %=
    
    // Comparison operators
    Equal,      // ==
    NotEqual,   // !=
    Greater,    // >
    GreaterEqual, // >=
    Less,       // <
    LessEqual,  // <=
    
    // Logical operators
    And,    // &&
    Or,     // ||
    Not,    // !
    
    // Control flow keywords
    If,
    Else,
    Elif,
    While,
    For,
    In,
    Break,
    Continue,
    Return,
    
    // Function declaration
    Fun,
    
    // Exception handling
    Try,
    Catch,
    Throw,
    Finally,
    
    // I/O keywords
    Show,
    Read,
    Exit,
    
    // Boolean literals
    True,
    False,
    Null,
    
    // Document Type Declaration
    DocumentType,  // type declaration (script, cli)
    
    // Module system
    Use,           // use module imports
    Export,        // export module items
    As,            // namespace alias
    From,          // module source
    
    // Debug and developer tools
    Debug,         // debug mode
    Assert,        // assertions
    Trace,         // execution tracing
    
    // Delimiters
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Comma,          // ,
    Dot,            // .
    Semicolon,      // ;
    Colon,          // :
    
    // Literals
    Identifier,     // Variable names, function names, etc.
    StringLiteral,  // "hello"
    NumberLiteral,  // 123, 3.14
    
    // Comments
    Comment,        // # Comment
    
    // Special
    EOF,            // End of file
    Illegal,        // Invalid token
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Variable declaration keywords
            TokenType::Let => write!(f, "LET"),
            TokenType::Take => write!(f, "TAKE"),
            TokenType::Hold => write!(f, "HOLD"),
            TokenType::Put => write!(f, "PUT"),
            
            // Mathematical operators
            TokenType::Plus => write!(f, "+"),
            TokenType::Minus => write!(f, "-"),
            TokenType::Asterisk => write!(f, "*"),
            TokenType::Slash => write!(f, "/"),
            TokenType::Percent => write!(f, "%"),
            TokenType::Power => write!(f, "**"),
            TokenType::FloorDiv => write!(f, "//"),
            
            // Assignment operators
            TokenType::Assign => write!(f, "="),
            TokenType::PlusAssign => write!(f, "+="),
            TokenType::MinusAssign => write!(f, "-="),
            TokenType::AsteriskAssign => write!(f, "*="),
            TokenType::SlashAssign => write!(f, "/="),
            TokenType::PercentAssign => write!(f, "%="),
            
            // Comparison operators
            TokenType::Equal => write!(f, "=="),
            TokenType::NotEqual => write!(f, "!="),
            TokenType::Greater => write!(f, ">"),
            TokenType::GreaterEqual => write!(f, ">="),
            TokenType::Less => write!(f, "<"),
            TokenType::LessEqual => write!(f, "<="),
            
            // Logical operators
            TokenType::And => write!(f, "&&"),
            TokenType::Or => write!(f, "||"),
            TokenType::Not => write!(f, "!"),
            
            // Control flow keywords
            TokenType::If => write!(f, "IF"),
            TokenType::Else => write!(f, "ELSE"),
            TokenType::Elif => write!(f, "ELIF"),
            TokenType::While => write!(f, "WHILE"),
            TokenType::For => write!(f, "FOR"),
            TokenType::In => write!(f, "IN"),
            TokenType::Break => write!(f, "BREAK"),
            TokenType::Continue => write!(f, "CONTINUE"),
            TokenType::Return => write!(f, "RETURN"),
            
            // Function declaration
            TokenType::Fun => write!(f, "FUN"),
            
            // Exception handling
            TokenType::Try => write!(f, "TRY"),
            TokenType::Catch => write!(f, "CATCH"),
            TokenType::Throw => write!(f, "THROW"),
            TokenType::Finally => write!(f, "FINALLY"),
            
            // I/O keywords
            TokenType::Show => write!(f, "SHOW"),
            TokenType::Read => write!(f, "READ"),
            TokenType::Exit => write!(f, "EXIT"),
            
            // Boolean literals
            TokenType::True => write!(f, "TRUE"),
            TokenType::False => write!(f, "FALSE"),
            TokenType::Null => write!(f, "NULL"),
            
            // Document Type Declaration
            TokenType::DocumentType => write!(f, "TYPE"),
            
            // Module system
            TokenType::Use => write!(f, "USE"),
            TokenType::Export => write!(f, "EXPORT"),
            TokenType::As => write!(f, "AS"),
            TokenType::From => write!(f, "FROM"),
            
            // Debug and developer tools
            TokenType::Debug => write!(f, "DEBUG"),
            TokenType::Assert => write!(f, "ASSERT"),
            TokenType::Trace => write!(f, "TRACE"),
            
            // Delimiters
            TokenType::LeftParen => write!(f, "("),
            TokenType::RightParen => write!(f, ")"),
            TokenType::LeftBrace => write!(f, "{{"),
            TokenType::RightBrace => write!(f, "}}"),
            TokenType::LeftBracket => write!(f, "["),
            TokenType::RightBracket => write!(f, "]"),
            TokenType::Comma => write!(f, ","),
            TokenType::Dot => write!(f, "."),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Colon => write!(f, ":"),
            
            // Literals
            TokenType::Identifier => write!(f, "IDENTIFIER"),
            TokenType::StringLiteral => write!(f, "STRING"),
            TokenType::NumberLiteral => write!(f, "NUMBER"),
            
            // Comments
            TokenType::Comment => write!(f, "COMMENT"),
            
            // Special
            TokenType::EOF => write!(f, "EOF"),
            TokenType::Illegal => write!(f, "ILLEGAL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub token_type: TokenType,
    pub literal: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(token_type: TokenType, literal: impl Into<String>, line: usize, column: usize) -> Self {
        Token {
            token_type,
            literal: literal.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:'{}' at {}:{}", self.token_type, self.literal, self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentType {
    Web,
    Script,
    Cli,
    Freestyle,
}

impl Token {
    pub fn is_valid_for_type(&self, doc_type: &DocumentType) -> bool {
        match doc_type {
            DocumentType::Freestyle => true, // Freestyle accepts all tokens
            DocumentType::Web => matches!(self.token_type, TokenType::DocumentType /* | other web tokens */),
            DocumentType::Script => matches!(self.token_type, TokenType::DocumentType /* | other script tokens */),
            DocumentType::Cli => matches!(self.token_type, TokenType::DocumentType /* | other cli tokens */),
        }
    }
}

// Helper function to lookup keywords
pub fn lookup_identifier(identifier: &str) -> TokenType {
    match identifier {
        // Variable declaration keywords
        "let" => TokenType::Let,
        "take" => TokenType::Take,
        "hold" => TokenType::Hold,
        "put" => TokenType::Put,
        
        // Control flow keywords
        "if" => TokenType::If,
        "else" => TokenType::Else,
        "elif" => TokenType::Elif,
        "while" => TokenType::While,
        "for" => TokenType::For,
        "in" => TokenType::In,
        "break" => TokenType::Break,
        "continue" => TokenType::Continue,
        "return" => TokenType::Return,
        
        // Function declaration
        "fun" => TokenType::Fun,
        
        // Exception handling
        "try" => TokenType::Try,
        "catch" => TokenType::Catch,
        "throw" => TokenType::Throw,
        "finally" => TokenType::Finally,
        
        // I/O keywords
        "show" => TokenType::Show,
        "read" => TokenType::Read,
        "exit" => TokenType::Exit,
        
        // Boolean literals
        "true" => TokenType::True,
        "false" => TokenType::False,
        "null" => TokenType::Null,
        
        // Document Type Declaration
        "type" => TokenType::DocumentType,
        
        // Module system
        "use" => TokenType::Use,
        "export" => TokenType::Export,
        "as" => TokenType::As,
        "from" => TokenType::From,
        
        // Debug and developer tools
        "debug" => TokenType::Debug,
        "assert" => TokenType::Assert,
        "trace" => TokenType::Trace,
        
        // If not a keyword, it's an identifier
        _ => TokenType::Identifier,
    }
}