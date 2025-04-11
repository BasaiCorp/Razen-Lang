use std::collections::HashMap;
use std::path::Path;

use crate::ast::{Program, Statement, Expression};
use crate::token::{Token, TokenType, DocumentType};
use crate::lexer::Lexer;

// Define parse error types
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    ExpectedDocumentType,
    InvalidDocumentType,
    InvalidTokenForType,
    UnknownVariable(String, Vec<String>), // Variable name and suggestions
    InvalidSyntax(String),
    ExpectedToken(TokenType, TokenType),  // Expected, Got
    // Add other error types as needed
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::ExpectedDocumentType => write!(f, "Expected document type declaration (web, script, cli, freestyle)"),
            ParseError::InvalidDocumentType => write!(f, "Invalid document type. Valid types are: web, script, cli, freestyle"),
            ParseError::InvalidTokenForType => write!(f, "Token not allowed for this document type"),
            ParseError::UnknownVariable(var, suggestions) => {
                write!(f, "Unknown variable: '{}'", var)?;
                if !suggestions.is_empty() {
                    write!(f, ". Did you mean: ")?;
                    for (i, suggestion) in suggestions.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "'{}'", suggestion)?;
                    }
                    write!(f, "?")?;
                }
                Ok(())
            },
            ParseError::InvalidSyntax(msg) => write!(f, "Invalid syntax: {}", msg),
            ParseError::ExpectedToken(expected, got) => {
                write!(f, "Expected token '{}', got '{}'", expected, got)
            },
        }
    }
}

// Define operator precedence levels
#[derive(PartialEq, PartialOrd, Debug)]
enum Precedence {
    Lowest,
    Assignment,  // =
    LogicalOr,   // ||
    LogicalAnd,  // &&
    Equals,      // ==, !=
    LessGreater, // >, <, >=, <=
    Sum,         // +, -
    Product,     // *, /, %
    Power,       // **
    Prefix,      // -X, !X
    Call,        // myFunction(X)
    Index,       // array[index]
}

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    peek_token: Token,
    errors: Vec<String>,
    // Maps for prefix and infix parsing functions
    prefix_parse_fns: HashMap<TokenType, fn(&mut Parser) -> Option<Expression>>,
    infix_parse_fns: HashMap<TokenType, fn(&mut Parser, Expression) -> Option<Expression>>,
    variables: HashMap<String, Expression>,
    known_variables: HashMap<String, String>, // Variable name -> description
    document_type: Option<DocumentType>,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();
        
        let mut parser = Parser {
            lexer,
            current_token,
            peek_token,
            errors: Vec::new(),
            prefix_parse_fns: HashMap::new(),
            infix_parse_fns: HashMap::new(),
            variables: HashMap::new(),
            known_variables: HashMap::new(),
            document_type: None,
        };
        
        // Load known variables from the language definition
        parser.load_known_variables();
        
        // Register prefix parse functions
        parser.register_prefix(TokenType::Identifier, Parser::parse_identifier);
        parser.register_prefix(TokenType::StringLiteral, Parser::parse_string_literal);
        parser.register_prefix(TokenType::NumberLiteral, Parser::parse_number_literal);
        parser.register_prefix(TokenType::True, Parser::parse_boolean_literal);
        parser.register_prefix(TokenType::False, Parser::parse_boolean_literal);
        parser.register_prefix(TokenType::Null, Parser::parse_null_literal);
        parser.register_prefix(TokenType::LeftParen, Parser::parse_grouped_expression);
        parser.register_prefix(TokenType::LeftBracket, Parser::parse_array_literal);
        parser.register_prefix(TokenType::LeftBrace, Parser::parse_map_literal);
        parser.register_prefix(TokenType::Minus, Parser::parse_prefix_expression);
        parser.register_prefix(TokenType::Not, Parser::parse_prefix_expression);
        
        // Register infix parse functions
        parser.register_infix(TokenType::Plus, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Minus, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Slash, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Asterisk, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Percent, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Power, Parser::parse_infix_expression);
        parser.register_infix(TokenType::FloorDiv, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Equal, Parser::parse_infix_expression);
        parser.register_infix(TokenType::NotEqual, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Less, Parser::parse_infix_expression);
        parser.register_infix(TokenType::LessEqual, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Greater, Parser::parse_infix_expression);
        parser.register_infix(TokenType::GreaterEqual, Parser::parse_infix_expression);
        parser.register_infix(TokenType::And, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Or, Parser::parse_infix_expression);
        parser.register_infix(TokenType::Assign, Parser::parse_assignment_expression);
        parser.register_infix(TokenType::PlusAssign, Parser::parse_assignment_expression);
        parser.register_infix(TokenType::MinusAssign, Parser::parse_assignment_expression);
        parser.register_infix(TokenType::AsteriskAssign, Parser::parse_assignment_expression);
        parser.register_infix(TokenType::SlashAssign, Parser::parse_assignment_expression);
        parser.register_infix(TokenType::PercentAssign, Parser::parse_assignment_expression);
        parser.register_infix(TokenType::LeftParen, Parser::parse_call_expression);
        parser.register_infix(TokenType::LeftBracket, Parser::parse_index_expression);
        
        parser
    }
    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        match Lexer::from_file(path) {
            Ok(lexer) => Ok(Parser::new(lexer)),
            Err(e) => Err(e),
        }
    }
    
    fn register_prefix(&mut self, token_type: TokenType, func: fn(&mut Parser) -> Option<Expression>) {
        self.prefix_parse_fns.insert(token_type, func);
    }
    
    fn register_infix(&mut self, token_type: TokenType, func: fn(&mut Parser, Expression) -> Option<Expression>) {
        self.infix_parse_fns.insert(token_type, func);
    }
    
    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.peek_token = self.lexer.next_token();
    }
    
    fn current_token_is(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }
    
    fn peek_token_is(&self, token_type: TokenType) -> bool {
        self.peek_token.token_type == token_type
    }
    
    fn expect_peek(&mut self, token_type: TokenType) -> bool {
        if self.peek_token_is(token_type.clone()) {
            self.next_token();
            true
        } else {
            self.peek_error(token_type);
            false
        }
    }
    
    fn peek_error(&mut self, token_type: TokenType) {
        let msg = format!(
            "Expected next token to be {:?}, got {:?} instead at line {}, column {}",
            token_type,
            self.peek_token.token_type,
            self.peek_token.line,
            self.peek_token.column
        );
        self.errors.push(msg);
    }
    
    pub fn get_errors(&self) -> &[String] {
        &self.errors
    }
    
    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::new();
        
        while !self.current_token_is(TokenType::EOF) {
            if let Some(stmt) = self.parse_statement() {
                program.statements.push(stmt);
            }
            self.next_token();
        }
        
        program
    }
    
    fn parse_statement(&mut self) -> Option<Statement> {
        match self.current_token.token_type {
            TokenType::Let | TokenType::Take | TokenType::Hold | TokenType::Put => self.parse_variable_declaration(),
            TokenType::Fun => self.parse_function_declaration(),
            TokenType::Return => self.parse_return_statement(),
            TokenType::If => self.parse_if_statement(),
            TokenType::While => self.parse_while_statement(),
            TokenType::For => self.parse_for_statement(),
            TokenType::Break => self.parse_break_statement(),
            TokenType::Continue => self.parse_continue_statement(),
            TokenType::Show => self.parse_show_statement(),
            TokenType::Read => self.parse_read_statement(),
            TokenType::Exit => self.parse_exit_statement(),
            TokenType::Try => self.parse_try_statement(),
            TokenType::Throw => self.parse_throw_statement(),
            TokenType::DocumentType => self.parse_document_type_declaration(),
            // Module system
            TokenType::Use => self.parse_module_import(),
            TokenType::Export => self.parse_module_export(),
            // Developer tools
            TokenType::Debug => self.parse_debug_statement(),
            TokenType::Assert => self.parse_assert_statement(),
            TokenType::Trace => self.parse_trace_statement(),
            TokenType::Comment => {
                // Skip comments and return None to continue parsing
                None
            },
            _ => self.parse_expression_statement(),
        }
    }
    
    fn parse_variable_declaration(&mut self) -> Option<Statement> {
        let var_type = self.current_token.literal.clone();
        
        // Check if the variable type is valid
        let valid_var_types = ["let", "take", "hold", "put"];
        if !valid_var_types.contains(&var_type.as_str()) {
            let mut suggestions = Vec::new();
            for vt in valid_var_types.iter() {
                if let Some(desc) = self.known_variables.get(*vt) {
                    suggestions.push(format!("{} ({})", vt, desc));
                } else {
                    suggestions.push((*vt).to_string());
                }
            }
            self.errors.push(format!("Invalid variable type: '{}'. Valid types are: {}", 
                var_type, suggestions.join(", ")));
            return None;
        }
        
        if !self.expect_peek(TokenType::Identifier) {
            self.errors.push("Expected variable name after variable type".to_string());
            return None;
        }
        
        let name = self.current_token.literal.clone();
        
        if !self.expect_peek(TokenType::Assign) {
            self.errors.push(format!("Expected '=' after variable name '{}'", name));
            return None;
        }
        
        self.next_token();
        
        let value = self.parse_expression(Precedence::Lowest);
        
        // Check if the value type matches the variable type
        if let Some(ref expr) = value {
            match var_type.as_str() {
                "let" => {
                    if !self.is_number_expression(expr) {
                        self.errors.push(format!(
                            "Type mismatch: 'let' expects a number value for variable '{}'. Use 'take' for strings, 'hold' for booleans, or 'put' for any type.", 
                            name
                        ));
                    }
                },
                "take" => {
                    if !self.is_string_expression(expr) {
                        self.errors.push(format!(
                            "Type mismatch: 'take' expects a string value for variable '{}'. Use 'let' for numbers, 'hold' for booleans, or 'put' for any type.", 
                            name
                        ));
                    }
                },
                "hold" => {
                    if !self.is_boolean_expression(expr) {
                        self.errors.push(format!(
                            "Type mismatch: 'hold' expects a boolean value for variable '{}'. Use 'let' for numbers, 'take' for strings, or 'put' for any type.", 
                            name
                        ));
                    }
                },
                _ => {} // 'put' accepts any type
            }
        }
        
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        self.variables.insert(name.clone(), value.clone().unwrap_or(Expression::Identifier("undefined".to_string())));
        
        Some(Statement::VariableDeclaration {
            var_type,
            name,
            value,
        })
    }
    
    fn parse_function_declaration(&mut self) -> Option<Statement> {
        if !self.expect_peek(TokenType::Identifier) {
            return None;
        }
        
        let name = self.current_token.literal.clone();
        
        if !self.expect_peek(TokenType::LeftParen) {
            return None;
        }
        
        let parameters = self.parse_function_parameters();
        
        if !self.expect_peek(TokenType::LeftBrace) {
            return None;
        }
        
        let body = self.parse_block_statement();
        
        Some(Statement::FunctionDeclaration {
            name,
            parameters,
            body,
        })
    }
    
    fn parse_function_parameters(&mut self) -> Vec<String> {
        let mut parameters = Vec::new();
        
        if self.peek_token_is(TokenType::RightParen) {
            self.next_token();
            return parameters;
        }
        
        self.next_token();
        
        parameters.push(self.current_token.literal.clone());
        
        while self.peek_token_is(TokenType::Comma) {
            self.next_token(); // Skip comma
            self.next_token(); // Move to next parameter
            parameters.push(self.current_token.literal.clone());
        }
        
        if !self.expect_peek(TokenType::RightParen) {
            return Vec::new();
        }
        
        parameters
    }
    
    fn parse_return_statement(&mut self) -> Option<Statement> {
        self.next_token();
        
        let value = if self.current_token_is(TokenType::Semicolon) {
            None
        } else {
            let expr = self.parse_expression(Precedence::Lowest)?;
            Some(expr)
        };
        
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ReturnStatement { value })
    }
    
    fn parse_expression_statement(&mut self) -> Option<Statement> {
        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ExpressionStatement { expression: expr })
    }
    
    fn parse_block_statement(&mut self) -> Vec<Statement> {
        let mut statements = Vec::new();
        
        self.next_token();
        
        while !self.current_token_is(TokenType::RightBrace) && !self.current_token_is(TokenType::EOF) {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.next_token();
        }
        
        statements
    }
    
    fn parse_if_statement(&mut self) -> Option<Statement> {
        if !self.expect_peek(TokenType::LeftParen) {
            return None;
        }
        
        self.next_token();
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        if !self.expect_peek(TokenType::RightParen) {
            return None;
        }
        
        if !self.expect_peek(TokenType::LeftBrace) {
            return None;
        }
        
        let consequence = self.parse_block_statement();
        
        let alternative = if self.peek_token_is(TokenType::Else) {
            self.next_token();
            
            if !self.expect_peek(TokenType::LeftBrace) {
                return None;
            }
            
            Some(self.parse_block_statement())
        } else {
            None
        };
        
        Some(Statement::IfStatement {
            condition,
            consequence,
            alternative,
        })
    }
    
    fn parse_while_statement(&mut self) -> Option<Statement> {
        if !self.expect_peek(TokenType::LeftParen) {
            return None;
        }
        
        self.next_token();
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        if !self.expect_peek(TokenType::RightParen) {
            return None;
        }
        
        if !self.expect_peek(TokenType::LeftBrace) {
            return None;
        }
        
        let body = self.parse_block_statement();
        
        Some(Statement::WhileStatement {
            condition,
            body,
        })
    }
    
    fn parse_for_statement(&mut self) -> Option<Statement> {
        if !self.expect_peek(TokenType::LeftParen) {
            return None;
        }
        
        self.next_token();
        let iterator = self.current_token.literal.clone();
        
        if !self.expect_peek(TokenType::In) {
            return None;
        }
        
        self.next_token();
        let iterable = self.parse_expression(Precedence::Lowest)?;
        
        if !self.expect_peek(TokenType::RightParen) {
            return None;
        }
        
        if !self.expect_peek(TokenType::LeftBrace) {
            return None;
        }
        
        let body = self.parse_block_statement();
        
        Some(Statement::ForStatement {
            iterator,
            iterable,
            body,
        })
    }
    
    fn parse_break_statement(&mut self) -> Option<Statement> {
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::BreakStatement)
    }
    
    fn parse_continue_statement(&mut self) -> Option<Statement> {
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ContinueStatement)
    }
    
    fn parse_show_statement(&mut self) -> Option<Statement> {
        self.next_token();
        
        let value = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ShowStatement { value })
    }
    
    fn parse_try_statement(&mut self) -> Option<Statement> {
        if !self.expect_peek(TokenType::LeftBrace) {
            return None;
        }
        
        let try_block = self.parse_block_statement();
        
        let catch_block = if self.peek_token_is(TokenType::Catch) {
            self.next_token();
            
            if !self.expect_peek(TokenType::LeftBrace) {
                return None;
            }
            
            Some(self.parse_block_statement())
        } else {
            None
        };
        
        let finally_block = if self.peek_token_is(TokenType::Finally) {
            self.next_token();
            
            if !self.expect_peek(TokenType::LeftBrace) {
                return None;
            }
            
            Some(self.parse_block_statement())
        } else {
            None
        };
        
        Some(Statement::TryStatement {
            try_block,
            catch_block,
            finally_block,
        })
    }
    
    fn parse_throw_statement(&mut self) -> Option<Statement> {
        self.next_token();
        
        let value = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ThrowStatement { value })
    }
    
    fn parse_read_statement(&mut self) -> Option<Statement> {
        if !self.expect_peek(TokenType::Identifier) {
            return None;
        }
        
        let name = self.current_token.literal.clone();
        
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ReadStatement { name })
    }
    
    fn parse_exit_statement(&mut self) -> Option<Statement> {
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ExitStatement)
    }
    
    /// Parse document type declaration (type web; type script; type cli;)
    fn parse_document_type_declaration(&mut self) -> Option<Statement> {
        // Consume the 'type' token
        self.next_token();
        
        // Get the document type (web, script, cli)
        let doc_type = if self.peek_token_is(TokenType::Identifier) {
            self.next_token();
            self.current_token.literal.clone()
        } else {
            self.errors.push(format!("Expected document type after 'type', got {:?}", self.peek_token.token_type));
            return None;
        };
        
        // Expect semicolon
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::DocumentTypeDeclaration { doc_type })
    }
    
    /// Parse module import statement (use name from "module"; or use name1, name2 from "module"; or use module as alias from "module";)
    fn parse_module_import(&mut self) -> Option<Statement> {
        // Skip 'use' token
        self.next_token();
        
        // Parse import names
        let mut names = Vec::new();
        let mut alias = None;
        
        // First name must be an identifier
        if !self.current_token_is(TokenType::Identifier) {
            self.errors.push(format!("Expected identifier after 'use', got {:?}", self.current_token.token_type));
            return None;
        }
        
        names.push(self.current_token.literal.clone());
        self.next_token();
        
        // Check for 'as' keyword for namespace alias
        if self.current_token_is(TokenType::As) {
            self.next_token(); // Skip 'as'
            
            if !self.current_token_is(TokenType::Identifier) {
                self.errors.push(format!("Expected identifier after 'as', got {:?}", self.current_token.token_type));
                return None;
            }
            
            alias = Some(self.current_token.literal.clone());
            self.next_token();
        } else {
            // Parse additional names if there's a comma
            while self.current_token_is(TokenType::Comma) {
                self.next_token(); // Skip comma
                
                if !self.current_token_is(TokenType::Identifier) {
                    self.errors.push(format!("Expected identifier after comma, got {:?}", self.current_token.token_type));
                    return None;
                }
                
                names.push(self.current_token.literal.clone());
                self.next_token();
            }
        }
        
        // Expect 'from' keyword
        if !self.current_token_is(TokenType::From) {
            self.errors.push(format!("Expected 'from' after import names, got {:?}", self.current_token.token_type));
            return None;
        }
        
        // Skip 'from'
        self.next_token();
        
        // Expect string literal for module path
        if !self.current_token_is(TokenType::StringLiteral) {
            self.errors.push(format!("Expected string literal for module path, got {:?}", self.current_token.token_type));
            return None;
        }
        
        let source = self.current_token.literal.clone();
        
        // Skip string literal
        self.next_token();
        
        // Expect semicolon
        if self.current_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ModuleImport { names, alias, source })
    }
    
    /// Parse module export statement (export name;)
    fn parse_module_export(&mut self) -> Option<Statement> {
        // Skip 'export' token
        self.next_token();
        
        if !self.current_token_is(TokenType::Identifier) {
            self.errors.push(format!("Expected identifier after 'export', got {:?}", self.current_token.token_type));
            return None;
        }
        
        let name = self.current_token.literal.clone();
        
        // Skip identifier
        self.next_token();
        
        // Expect semicolon
        if self.current_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::ModuleExport { name })
    }
    
    /// Parse debug statement (debug expression;)
    fn parse_debug_statement(&mut self) -> Option<Statement> {
        // Skip 'debug' token
        self.next_token();
        
        let value = match self.parse_expression(Precedence::Lowest) {
            Some(expr) => expr,
            None => return None,
        };
        
        // Expect semicolon
        if self.current_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::DebugStatement { value })
    }
    
    /// Parse assert statement (assert(condition, message?);)
    fn parse_assert_statement(&mut self) -> Option<Statement> {
        // Skip 'assert' token
        self.next_token();
        
        // Expect left parenthesis
        if !self.expect_peek(TokenType::LeftParen) {
            return None;
        }
        
        // Skip left parenthesis and move to condition
        self.next_token();
        
        let condition = match self.parse_expression(Precedence::Lowest) {
            Some(expr) => expr,
            None => return None,
        };
        
        let mut message = None;
        
        // Check for comma (optional message)
        if self.peek_token_is(TokenType::Comma) {
            self.next_token(); // Move to comma
            self.next_token(); // Move past comma to message expression
            
            message = match self.parse_expression(Precedence::Lowest) {
                Some(expr) => Some(expr),
                None => return None,
            };
        }
        
        // Expect right parenthesis
        if !self.expect_peek(TokenType::RightParen) {
            return None;
        }
        
        // Expect semicolon
        if self.peek_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::AssertStatement { condition, message })
    }
    
    /// Parse trace statement (trace expression;)
    fn parse_trace_statement(&mut self) -> Option<Statement> {
        // Skip 'trace' token
        self.next_token();
        
        let value = match self.parse_expression(Precedence::Lowest) {
            Some(expr) => expr,
            None => return None,
        };
        
        // Expect semicolon
        if self.current_token_is(TokenType::Semicolon) {
            self.next_token();
        }
        
        Some(Statement::TraceStatement { value })
    }
    
    fn parse_expression(&mut self, precedence: Precedence) -> Option<Expression> {
        // Skip comments
        if self.current_token.token_type == TokenType::Comment {
            return None;
        }
        
        // Try to get a prefix parse function for the current token
        let prefix = self.prefix_parse_fns.get(&self.current_token.token_type).cloned();
        
        if prefix.is_none() {
            self.errors.push(format!(
                "No prefix parse function for {:?} found at line {}, column {}",
                self.current_token.token_type,
                self.current_token.line,
                self.current_token.column
            ));
            return None;
        }
        
        let mut left_exp = prefix.unwrap()(self)?;
        
        while !self.peek_token_is(TokenType::Semicolon) && precedence < self.peek_precedence() {
            let infix = self.infix_parse_fns.get(&self.peek_token.token_type).cloned();
            
            if infix.is_none() {
                return Some(left_exp);
            }
            
            self.next_token();
            
            left_exp = infix.unwrap()(self, left_exp)?;
        }
        
        Some(left_exp)
    }
    
    fn parse_identifier(&mut self) -> Option<Expression> {
        let name = self.current_token.literal.clone();
        
        if let Some(expr) = self.variables.get(&name) {
            Some(expr.clone())
        } else {
            let suggestions = self.suggest_variables(&name);
            self.errors.push(ParseError::UnknownVariable(name, suggestions).to_string());
            None
        }
    }
    
    fn parse_string_literal(&mut self) -> Option<Expression> {
        Some(Expression::StringLiteral(self.current_token.literal.clone()))
    }
    
    fn parse_number_literal(&mut self) -> Option<Expression> {
        match self.current_token.literal.parse::<f64>() {
            Ok(value) => Some(Expression::NumberLiteral(value)),
            Err(_) => {
                self.errors.push(format!(
                    "Could not parse {} as number at line {}, column {}",
                    self.current_token.literal,
                    self.current_token.line,
                    self.current_token.column
                ));
                None
            }
        }
    }
    
    fn parse_boolean_literal(&mut self) -> Option<Expression> {
        Some(Expression::BooleanLiteral(self.current_token_is(TokenType::True)))
    }
    
    fn parse_null_literal(&mut self) -> Option<Expression> {
        Some(Expression::NullLiteral)
    }
    
    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let operator = self.current_token.literal.clone();
        
        self.next_token();
        
        let right = self.parse_expression(Precedence::Prefix)?;
        
        Some(Expression::PrefixExpression {
            operator,
            right: Box::new(right),
        })
    }
    
    fn parse_infix_expression(&mut self, left: Expression) -> Option<Expression> {
        let operator = self.current_token.literal.clone();
        let precedence = self.current_precedence();
        
        self.next_token();
        
        let right = self.parse_expression(precedence)?;
        
        Some(Expression::InfixExpression {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }
    
    fn parse_assignment_expression(&mut self, left: Expression) -> Option<Expression> {
        let operator = self.current_token.literal.clone();
        
        self.next_token();
        
        let right = self.parse_expression(Precedence::Lowest)?;
        
        Some(Expression::AssignmentExpression {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }
    
    fn parse_grouped_expression(&mut self) -> Option<Expression> {
        self.next_token();
        
        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if !self.expect_peek(TokenType::RightParen) {
            return None;
        }
        
        Some(expr)
    }
    
    fn parse_array_literal(&mut self) -> Option<Expression> {
        let elements = self.parse_expression_list(TokenType::RightBracket)?;
        
        Some(Expression::ArrayLiteral { elements })
    }
    
    fn parse_map_literal(&mut self) -> Option<Expression> {
        let mut pairs = Vec::new();
        
        while !self.peek_token_is(TokenType::RightBrace) {
            self.next_token();
            
            let key = self.parse_expression(Precedence::Lowest)?;
            
            if !self.expect_peek(TokenType::Colon) {
                return None;
            }
            
            self.next_token();
            
            let value = self.parse_expression(Precedence::Lowest)?;
            
            pairs.push((key, value));
            
            if !self.peek_token_is(TokenType::RightBrace) && !self.expect_peek(TokenType::Comma) {
                return None;
            }
        }
        
        if !self.expect_peek(TokenType::RightBrace) {
            return None;
        }
        
        Some(Expression::MapLiteral { pairs })
    }
    
    fn parse_call_expression(&mut self, function: Expression) -> Option<Expression> {
        let arguments = self.parse_expression_list(TokenType::RightParen)?;
        
        Some(Expression::CallExpression {
            function: Box::new(function),
            arguments,
        })
    }
    
    fn parse_index_expression(&mut self, left: Expression) -> Option<Expression> {
        self.next_token();
        
        let index = self.parse_expression(Precedence::Lowest)?;
        
        if !self.expect_peek(TokenType::RightBracket) {
            return None;
        }
        
        Some(Expression::IndexExpression {
            left: Box::new(left),
            index: Box::new(index),
        })
    }
    
    fn parse_expression_list(&mut self, end: TokenType) -> Option<Vec<Expression>> {
        let mut list = Vec::new();
        
        if self.peek_token_is(end.clone()) {
            self.next_token();
            return Some(list);
        }
        
        self.next_token();
        
        list.push(self.parse_expression(Precedence::Lowest)?);
        
        while self.peek_token_is(TokenType::Comma) {
            self.next_token();
            self.next_token();
            list.push(self.parse_expression(Precedence::Lowest)?);
        }
        
        if !self.expect_peek(end) {
            return None;
        }
        
        Some(list)
    }
    
    fn current_precedence(&self) -> Precedence {
        Self::token_precedence(&self.current_token.token_type)
    }
    
    fn peek_precedence(&self) -> Precedence {
        Self::token_precedence(&self.peek_token.token_type)
    }
    
    fn token_precedence(token_type: &TokenType) -> Precedence {
        match token_type {
            TokenType::Assign | TokenType::PlusAssign | TokenType::MinusAssign | 
            TokenType::AsteriskAssign | TokenType::SlashAssign | TokenType::PercentAssign => Precedence::Assignment,
            TokenType::Or => Precedence::LogicalOr,
            TokenType::And => Precedence::LogicalAnd,
            TokenType::Equal | TokenType::NotEqual => Precedence::Equals,
            TokenType::Less | TokenType::LessEqual | TokenType::Greater | TokenType::GreaterEqual => Precedence::LessGreater,
            TokenType::Plus | TokenType::Minus => Precedence::Sum,
            TokenType::Slash | TokenType::Asterisk | TokenType::Percent | TokenType::FloorDiv => Precedence::Product,
            TokenType::Power => Precedence::Power,
            TokenType::LeftParen => Precedence::Call,
            TokenType::LeftBracket => Precedence::Index,
            _ => Precedence::Lowest,
        }
    }

    fn suggest_variables(&self, name: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let name_lower = name.to_lowercase();
        
        // Helper function to calculate Levenshtein distance between two strings
        fn levenshtein_distance(a: &str, b: &str) -> usize {
            let a_len = a.chars().count();
            let b_len = b.chars().count();
            
            // Create a matrix of size (a_len+1) x (b_len+1)
            let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];
            
            // Initialize the first row and column
            for i in 0..=a_len {
                matrix[i][0] = i;
            }
            for j in 0..=b_len {
                matrix[0][j] = j;
            }
            
            // Fill the matrix
            let a_chars: Vec<char> = a.chars().collect();
            let b_chars: Vec<char> = b.chars().collect();
            
            for i in 1..=a_len {
                for j in 1..=b_len {
                    let cost = if a_chars[i-1] == b_chars[j-1] { 0 } else { 1 };
                    
                    matrix[i][j] = std::cmp::min(
                        matrix[i-1][j] + 1,                 // deletion
                        std::cmp::min(
                            matrix[i][j-1] + 1,             // insertion
                            matrix[i-1][j-1] + cost         // substitution
                        )
                    );
                }
            }
            
            matrix[a_len][b_len]
        }
        
        // Collect all variables with their Levenshtein distance
        let mut candidates = Vec::new();
        
        // Check variables in current scope
        for var in self.variables.keys() {
            let var_lower = var.to_lowercase();
            let distance = levenshtein_distance(&name_lower, &var_lower);
            
            // Only suggest if the distance is reasonable (max 3 edits or contains the name)
            if distance <= 3 || var_lower.contains(&name_lower) {
                candidates.push((var.clone(), distance));
            }
        }
        
        // Check known variables from language definition
        for var in self.known_variables.keys() {
            let var_lower = var.to_lowercase();
            let distance = levenshtein_distance(&name_lower, &var_lower);
            
            // Only suggest if the distance is reasonable (max 3 edits or contains the name)
            if distance <= 3 || var_lower.contains(&name_lower) {
                candidates.push((var.clone(), distance));
            }
        }
        
        // Sort by distance (closest first)
        candidates.sort_by_key(|(_, distance)| *distance);
        
        // Take top 3 suggestions
        for (var, _) in candidates.iter().take(3) {
            // Include description if available
            if let Some(desc) = self.known_variables.get(var) {
                suggestions.push(format!("{} ({})", var, desc));
            } else {
                suggestions.push(var.clone());
            }
        }
        
        suggestions
    }

    fn load_known_variables(&mut self) {
        // Try to load variables from the variables.rzn file
        let variables_path = Path::new("properties/variables.rzn");
        let syntax_path = Path::new("properties/syntax.rzn");
        
        // Load from variables.rzn
        if let Ok(content) = std::fs::read_to_string(variables_path) {
            for line in content.lines() {
                // Skip comments and empty lines
                if line.trim().starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                
                // Parse lines in format: "variable => description"
                if let Some(idx) = line.find("=>") {
                    let var_name = line[..idx].trim().to_string();
                    let description = line[idx+2..].trim().to_string();
                    if !var_name.is_empty() {
                        self.known_variables.insert(var_name, description);
                    }
                }
            }
        }
        
        // Load from syntax.rzn
        if let Ok(content) = std::fs::read_to_string(syntax_path) {
            for line in content.lines() {
                // Skip comments and empty lines
                if line.trim().starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                
                // Parse VAR: lines in format: "VAR: variable => description"
                if line.trim().starts_with("VAR:") {
                    let line = line.trim()[4..].trim();
                    if let Some(idx) = line.find("=>") {
                        let var_name = line[..idx].trim().to_string();
                        let description = line[idx+2..].trim().to_string();
                        if !var_name.is_empty() {
                            self.known_variables.insert(var_name, description);
                        }
                    }
                }
                
                // Parse FUNC: lines in format: "FUNC: function(params) => description"
                if line.trim().starts_with("FUNC:") {
                    let line = line.trim()[5..].trim();
                    if let Some(idx) = line.find("=>") {
                        let func_decl = line[..idx].trim();
                        if let Some(paren_idx) = func_decl.find('(') {
                            let func_name = func_decl[..paren_idx].trim().to_string();
                            let description = line[idx+2..].trim().to_string();
                            if !func_name.is_empty() {
                                self.known_variables.insert(func_name, description);
                            }
                        }
                    }
                }
            }
        }
        
        // If no variables were loaded, add some defaults
        if self.known_variables.is_empty() {
            self.known_variables.insert("let".to_string(), "For declaring numeric variables".to_string());
            self.known_variables.insert("take".to_string(), "For declaring string variables".to_string());
            self.known_variables.insert("hold".to_string(), "For declaring boolean variables".to_string());
            self.known_variables.insert("put".to_string(), "For declaring variables of any type".to_string());
            self.known_variables.insert("type".to_string(), "For declaring document structure".to_string());
        }
    }

    fn is_boolean_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::BooleanLiteral(_) => true,
            Expression::Identifier(name) => {
                // Check if the identifier refers to a boolean variable
                if let Some(Expression::BooleanLiteral(_)) = self.variables.get(name) {
                    true
                } else {
                    false
                }
            },
            Expression::InfixExpression { left, operator, right } => {
                match operator.as_str() {
                    "==" | "!=" | ">" | ">=" | "<" | "<=" | "&&" | "||" => true,
                    _ => false,
                }
            },
            Expression::PrefixExpression { operator, .. } => {
                operator == "!"
            },
            _ => false,
        }
    }
    
    fn is_number_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::NumberLiteral(_) => true,
            Expression::Identifier(name) => {
                // Check if the identifier refers to a number variable
                if let Some(Expression::NumberLiteral(_)) = self.variables.get(name) {
                    true
                } else {
                    false
                }
            },
            Expression::InfixExpression { left, operator, right } => {
                match operator.as_str() {
                    "+" | "-" | "*" | "/" | "%" | "**" | "//" => {
                        self.is_number_expression(left) && self.is_number_expression(right)
                    },
                    _ => false,
                }
            },
            Expression::PrefixExpression { operator, right } => {
                if operator == "-" {
                    self.is_number_expression(right)
                } else {
                    false
                }
            },
            _ => false,
        }
    }
    
    fn is_string_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::StringLiteral(_) => true,
            Expression::Identifier(name) => {
                // Check if the identifier refers to a string variable
                if let Some(Expression::StringLiteral(_)) = self.variables.get(name) {
                    true
                } else {
                    false
                }
            },
            Expression::InfixExpression { left, operator, right } => {
                if operator == "+" {
                    self.is_string_expression(left) || self.is_string_expression(right)
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    pub fn parse_document_type(&mut self) -> Result<DocumentType, ParseError> {
        match &self.current_token {
            Token { token_type: TokenType::Identifier, literal, .. } => match literal.as_str() {
                "web" => Ok(DocumentType::Web),
                "script" => Ok(DocumentType::Script),
                "cli" => Ok(DocumentType::Cli),
                "freestyle" => Ok(DocumentType::Freestyle),
                _ => Err(ParseError::InvalidDocumentType),
            },
            _ => Err(ParseError::ExpectedDocumentType),
        }
    }

    pub fn validate_tokens(&self, doc_type: &DocumentType) -> Result<(), ParseError> {
        // Create a clone of the lexer to tokenize all without affecting the parser state
        let mut lexer_clone = self.lexer.clone();
        let tokens = lexer_clone.tokenize_all();
        
        for token in tokens {
            if !token.is_valid_for_type(doc_type) {
                return Err(ParseError::InvalidTokenForType);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_variable_declaration() {
        let input = "let x = 5;";
        let lexer = Lexer::new(input.to_string());
        let mut parser = Parser::new(lexer);
        
        let program = parser.parse_program();
        
        assert_eq!(parser.get_errors().len(), 0, "Parser errors: {:?}", parser.get_errors());
        assert_eq!(program.statements.len(), 1);
        
        match &program.statements[0] {
            Statement::VariableDeclaration { var_type, name, value } => {
                assert_eq!(var_type, "let");
                assert_eq!(name, "x");
                
                match value {
                    Some(Expression::NumberLiteral(val)) => assert_eq!(*val, 5.0),
                    _ => panic!("Expected NumberLiteral, got {:?}", value),
                }
            },
            _ => panic!("Expected VariableDeclaration, got {:?}", program.statements[0]),
        }
    }
    
    #[test]
    fn test_function_declaration() {
        let input = "fun add(x, y) { return x + y; }";
        let lexer = Lexer::new(input.to_string());
        let mut parser = Parser::new(lexer);
        
        let program = parser.parse_program();
        
        assert_eq!(parser.get_errors().len(), 0, "Parser errors: {:?}", parser.get_errors());
        assert_eq!(program.statements.len(), 1);
        
        match &program.statements[0] {
            Statement::FunctionDeclaration { name, parameters, body } => {
                assert_eq!(name, "add");
                assert_eq!(parameters, &vec!["x".to_string(), "y".to_string()]);
                assert_eq!(body.len(), 1);
                
                match &body[0] {
                    Statement::ReturnStatement { value } => {
                        match value {
                            Some(Expression::InfixExpression { left, operator, right }) => {
                                match **left {
                                    Expression::Identifier(ref id) => assert_eq!(id, "x"),
                                    _ => panic!("Expected Identifier, got {:?}", left),
                                }
                                
                                assert_eq!(operator, "+");
                                
                                match **right {
                                    Expression::Identifier(ref id) => assert_eq!(id, "y"),
                                    _ => panic!("Expected Identifier, got {:?}", right),
                                }
                            },
                            _ => panic!("Expected InfixExpression, got {:?}", value),
                        }
                    },
                    _ => panic!("Expected ReturnStatement, got {:?}", body[0]),
                }
            },
            _ => panic!("Expected FunctionDeclaration, got {:?}", program.statements[0]),
        }
    }
}