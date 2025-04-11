use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::ast::{Program, Statement, Expression};
use crate::parser::Parser;

// Intermediate representation for code generation
#[derive(Debug, Clone)]
enum IR {
    // Stack operations
    PushNumber(f64),
    PushString(String),
    PushBoolean(bool),
    PushNull,
    Pop,
    Dup,
    Swap,
    
    // Memory operations
    StoreVar(String),
    LoadVar(String),
    SetGlobal(String),  // Add missing SetGlobal variant
    
    // Arithmetic operations
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    FloorDiv,
    Negate,
    
    // Comparison operations
    Equal,
    NotEqual,
    GreaterThan,
    GreaterEqual,
    LessThan,
    LessEqual,
    
    // Logical operations
    And,
    Or,
    Not,
    
    // Control flow
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    Call(String, usize),  // function name, arg count
    Return,
    
    // I/O operations
    Print,
    ReadInput,
    Exit,
    
    // Array operations
    CreateArray(usize),
    GetIndex,
    SetIndex,
    
    // Map operations
    CreateMap(usize),
    GetKey,
    SetKey,
    
    // Function definition
    DefineFunction(String, usize),  // function name, address
    
    // Labels for jumps
    Label(String),
}

// Symbol table for variable and function tracking
#[derive(Debug, Clone)]
struct SymbolTable {
    symbols: HashMap<String, usize>,
    parent: Option<Box<SymbolTable>>,
    next_index: usize,
}

impl SymbolTable {
    fn new() -> Self {
        SymbolTable {
            symbols: HashMap::new(),
            parent: None,
            next_index: 0,
        }
    }
    
    fn new_enclosed(parent: SymbolTable) -> Self {
        SymbolTable {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
            next_index: 0,
        }
    }
    
    fn define(&mut self, name: &str) -> usize {
        let index = self.next_index;
        self.symbols.insert(name.to_string(), index);
        self.next_index += 1;
        index
    }
    
    fn resolve(&self, name: &str) -> Option<usize> {
        match self.symbols.get(name) {
            Some(index) => Some(*index),
            None => {
                if let Some(parent) = &self.parent {
                    parent.resolve(name)
                } else {
                    None
                }
            }
        }
    }
}

// Function table for tracking function definitions
#[derive(Debug, Clone)]
struct FunctionTable {
    functions: HashMap<String, usize>,  // function name -> address in IR
}

impl FunctionTable {
    fn new() -> Self {
        FunctionTable {
            functions: HashMap::new(),
        }
    }
    
    fn define(&mut self, name: &str, address: usize) {
        self.functions.insert(name.to_string(), address);
    }
    
    fn resolve(&self, name: &str) -> Option<usize> {
        self.functions.get(name).copied()
    }
}

// Compiler for translating AST to machine code
pub struct Compiler {
    ir: Vec<IR>,
    symbol_table: SymbolTable,
    function_table: FunctionTable,
    current_function: Option<String>,
    break_stack: Vec<Vec<usize>>,    // Stack of break statement positions for nested loops
    continue_stack: Vec<Vec<usize>>, // Stack of continue statement positions for nested loops
    label_counter: usize,            // Counter for generating unique labels
    clean_output: bool,              // Flag to only show program output
    errors: Vec<String>,            // Compilation errors
    variable_types: HashMap<String, String>, // Track variable types (name -> type)
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            ir: Vec::new(),
            symbol_table: SymbolTable::new(),
            function_table: FunctionTable::new(),
            current_function: None,
            break_stack: Vec::new(),
            continue_stack: Vec::new(),
            label_counter: 0,
            clean_output: false,
            errors: Vec::new(),
            variable_types: HashMap::new(),
        }
    }
    
    // Set clean output mode
    pub fn set_clean_output(&mut self, clean: bool) {
        self.clean_output = clean;
    }
    
    // Helper methods for type checking
    fn is_number_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::NumberLiteral(_) => true,
            Expression::Identifier(name) => {
                // Check if the identifier refers to a variable of type 'let'
                self.variable_types.get(name).map_or(false, |var_type| var_type == "let")
            },
            Expression::PrefixExpression { right, .. } => self.is_number_expression(right),
            Expression::InfixExpression { left, right, operator, .. } => {
                // Most infix operations with numbers result in numbers
                match operator.as_str() {
                    "+" | "-" | "*" | "/" | "%" | "**" | "//" => {
                        self.is_number_expression(left) && self.is_number_expression(right)
                    },
                    _ => false,
                }
            },
            Expression::CallExpression { function, .. } => {
                // Check if it's a function known to return numbers
                if let Expression::Identifier(name) = &**function {
                    matches!(name.as_str(), 
                        "plus" | "minus" | "times" | "by" | "mod" | "power" | 
                        "round" | "sqrt" | "abs" | "size" | "count")
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
                // Check if the identifier refers to a variable of type 'take'
                self.variable_types.get(name).map_or(false, |var_type| var_type == "take")
            },
            Expression::InfixExpression { left, right, operator, .. } => {
                // String concatenation
                operator == "+" && (self.is_string_expression(left) || self.is_string_expression(right))
            },
            Expression::CallExpression { function, .. } => {
                // Check if it's a function known to return strings
                if let Expression::Identifier(name) = &**function {
                    matches!(name.as_str(), 
                        "join" | "big" | "small" | "trim" | "replace" | 
                        "date" | "read_file")
                } else {
                    false
                }
            },
            _ => false,
        }
    }
    
    fn is_boolean_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::BooleanLiteral(_) => true,
            Expression::Identifier(name) => {
                // Check if the identifier refers to a variable of type 'hold'
                self.variable_types.get(name).map_or(false, |var_type| var_type == "hold")
            },
            Expression::PrefixExpression { operator, right } => {
                operator == "!" && self.is_boolean_expression(right)
            },
            Expression::InfixExpression { operator, .. } => {
                // Comparison and logical operators return booleans
                matches!(operator.as_str(), 
                    "==" | "!=" | ">" | ">=" | "<" | "<=" | "&&" | "||")
            },
            Expression::CallExpression { function, .. } => {
                // Check if it's a function known to return booleans
                if let Expression::Identifier(name) = &**function {
                    matches!(name.as_str(), "contains")
                } else {
                    false
                }
            },
            _ => false,
        }
    }
    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        match Parser::from_file(path) {
            Ok(mut parser) => {
                let program = parser.parse_program();
                if !parser.get_errors().is_empty() {
                    return Err(format!("Parser errors: {:?}", parser.get_errors()));
                }
                
                let mut compiler = Compiler::new();
                
                // Check for clean output flag in environment
                if std::env::args().any(|arg| arg == "--clean-output") {
                    compiler.set_clean_output(true);
                }
                
                compiler.compile_program(program);
                Ok(compiler)
            },
            Err(e) => Err(e),
        }
    }
    
    fn generate_label(&mut self, prefix: &str) -> String {
        let label = format!("{}{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }
    
    fn emit(&mut self, code: IR) -> usize {
        let pos = self.ir.len();
        self.ir.push(code);
        pos
    }
    
    fn emit_label(&mut self, label: &str) -> usize {
        self.emit(IR::Label(label.to_string()))
    }
    
    fn replace_instruction(&mut self, pos: usize, code: IR) {
        self.ir[pos] = code;
    }
    
    fn enter_scope(&mut self) {
        let new_table = SymbolTable::new_enclosed(self.symbol_table.clone());
        self.symbol_table = new_table;
    }
    
    fn leave_scope(&mut self) {
        if let Some(parent) = self.symbol_table.parent.take() {
            self.symbol_table = *parent;
        }
    }
    
    fn enter_loop(&mut self) {
        self.break_stack.push(Vec::new());
        self.continue_stack.push(Vec::new());
    }
    
    fn leave_loop(&mut self, loop_start: usize, loop_end: usize) {
        // Patch all break statements
        if let Some(breaks) = self.break_stack.pop() {
            for pos in breaks {
                self.replace_instruction(pos, IR::Jump(loop_end));
            }
        }
        
        // Patch all continue statements
        if let Some(continues) = self.continue_stack.pop() {
            for pos in continues {
                self.replace_instruction(pos, IR::Jump(loop_start));
            }
        }
    }
    
    fn define_builtins(&mut self) {
        // Define built-in functions like print, math operations, etc.
        self.symbol_table.define("print");
        self.symbol_table.define("read");
        self.symbol_table.define("len");
        self.symbol_table.define("append");
        self.symbol_table.define("remove");
        
        // Module system built-ins
        self.symbol_table.define("__import_symbol");
        self.symbol_table.define("__import_module");
        self.symbol_table.define("__export_symbol");
        
        // Developer tools built-ins
        self.symbol_table.define("__debug");
        self.symbol_table.define("__assert");
        self.symbol_table.define("__assert_with_message");
        self.symbol_table.define("__trace");
        
        // Standard library functions
        self.symbol_table.define("floor");       // Math functions
        self.symbol_table.define("ceil");
        self.symbol_table.define("round");
        self.symbol_table.define("sin");
        self.symbol_table.define("cos");
        self.symbol_table.define("tan");
        self.symbol_table.define("sqrt");
        self.symbol_table.define("random");
        
        self.symbol_table.define("format");      // String functions
        self.symbol_table.define("substring");
        self.symbol_table.define("uppercase");
        self.symbol_table.define("lowercase");
        self.symbol_table.define("trim");
        self.symbol_table.define("replace");
        
        self.symbol_table.define("map");         // Array functions
        self.symbol_table.define("filter");
        self.symbol_table.define("reduce");
        self.symbol_table.define("join");
        self.symbol_table.define("split");
        
        self.symbol_table.define("now");         // Time functions
        self.symbol_table.define("format_date");
        self.symbol_table.define("sleep");
        
        self.symbol_table.define("parse_json");  // JSON functions
        self.symbol_table.define("stringify_json");
        
        self.symbol_table.define("read_file");   // File I/O functions
        self.symbol_table.define("write_file");
        self.symbol_table.define("append_file");
        
        self.symbol_table.define("get_args");    // Scripting functions
        self.symbol_table.define("get_env");
        self.symbol_table.define("set_env");
        self.symbol_table.define("run_command");
        self.symbol_table.define("exit_with");
        
        self.symbol_table.define("create_dir");  // File system functions
        self.symbol_table.define("remove_dir");
        self.symbol_table.define("list_dir");
        self.symbol_table.define("is_file");
        self.symbol_table.define("is_dir");
        self.symbol_table.define("file_exists");
        self.symbol_table.define("copy_file");
        self.symbol_table.define("move_file");
        self.symbol_table.define("delete_file");
        
        self.symbol_table.define("join_path");   // Path manipulation functions
        self.symbol_table.define("basename");
        self.symbol_table.define("dirname");
        self.symbol_table.define("absolute_path");
    }
    
    pub fn compile_program(&mut self, program: Program) {
        // Define built-in functions
        self.define_builtins();
        
        // First pass: register all functions
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { name, .. } = stmt {
                self.symbol_table.define(name);
            }
        }
        
        // Second pass: compile all statements
        for stmt in program.statements {
            self.compile_statement(stmt);
        }
    }
    
    fn compile_statement(&mut self, stmt: Statement) {
        match stmt {
            Statement::VariableDeclaration { var_type, name, value } => {
                self.compile_variable_declaration(var_type, name, value);
            },
            Statement::FunctionDeclaration { name, parameters, body } => {
                self.compile_function_declaration(name, parameters, body);
            },
            Statement::ReturnStatement { value } => {
                self.compile_return_statement(value);
            },
            Statement::ExpressionStatement { expression } => {
                self.compile_expression(expression);
                // Discard the result of the expression
                self.emit(IR::Pop);
            },
            Statement::BlockStatement { statements } => {
                self.compile_block_statement(statements);
            },
            Statement::IfStatement { condition, consequence, alternative } => {
                self.compile_if_statement(condition, consequence, alternative);
            },
            // Module system statements
            Statement::ModuleImport { names, alias, source } => {
                self.compile_module_import(names, alias, source);
            },
            Statement::ModuleExport { name } => {
                self.compile_module_export(name);
            },
            // Developer tools statements
            Statement::DebugStatement { value } => {
                self.compile_debug_statement(value);
            },
            Statement::AssertStatement { condition, message } => {
                self.compile_assert_statement(condition, message);
            },
            Statement::TraceStatement { value } => {
                self.compile_trace_statement(value);
            },
            Statement::WhileStatement { condition, body } => {
                self.compile_while_statement(condition, body);
            },
            Statement::ForStatement { iterator, iterable, body } => {
                self.compile_for_statement(iterator, iterable, body);
            },
            Statement::BreakStatement => {
                self.compile_break_statement();
            },
            Statement::ContinueStatement => {
                self.compile_continue_statement();
            },
            Statement::ShowStatement { value } => {
                self.compile_show_statement(value);
            },
            Statement::TryStatement { try_block, catch_block, finally_block } => {
                self.compile_try_statement(try_block, catch_block, finally_block);
            },
            Statement::ThrowStatement { value } => {
                self.compile_throw_statement(value);
            },
            Statement::ReadStatement { name } => {
                self.compile_read_statement(name);
            },
            Statement::ExitStatement => {
                self.compile_exit_statement();
            },
            Statement::DocumentTypeDeclaration { doc_type } => {
                self.compile_document_type_declaration(doc_type);
            },
            // Module system
            Statement::ModuleImport { names, alias, source } => {
                self.compile_module_import(names, alias, source);
            },
            Statement::ModuleExport { name } => {
                self.compile_module_export(name);
            },
            // Developer tools
            Statement::DebugStatement { value } => {
                self.compile_debug_statement(value);
            },
            Statement::AssertStatement { condition, message } => {
                self.compile_assert_statement(condition, message);
            },
            Statement::TraceStatement { value } => {
                self.compile_trace_statement(value);
            },
        }
    }
    
    fn compile_variable_declaration(&mut self, var_type: String, name: String, value: Option<Expression>) {
        // Define the variable in the symbol table
        self.symbol_table.define(&name);
        
        // Store the variable type for future type checking
        self.variable_types.insert(name.clone(), var_type.clone());
        
        // Compile the initializer expression if it exists
        if let Some(expr) = value.clone() {
            // Type checking based on variable type
            match var_type.as_str() {
                "let" => {
                    // Check that the value is a number
                    if !self.is_number_expression(&expr) {
                        self.errors.push(format!("Type error: 'let' variables can only be used with numeric values, but '{}' was assigned a non-numeric value", name));
                    }
                },
                "take" => {
                    // Check that the value is a string
                    if !self.is_string_expression(&expr) {
                        self.errors.push(format!("Type error: 'take' variables can only be used with string values, but '{}' was assigned a non-string value", name));
                    }
                },
                "hold" => {
                    // Check that the value is a boolean
                    if !self.is_boolean_expression(&expr) {
                        self.errors.push(format!("Type error: 'hold' variables can only be used with boolean values, but '{}' was assigned a non-boolean value", name));
                    }
                },
                "put" => {
                    // 'put' can hold any type, so no type checking needed
                },
                _ => {
                    self.errors.push(format!("Unknown variable type: {}", var_type));
                }
            }
            
            self.compile_expression(expr);
        } else {
            // If no initializer, push null as the default value
            self.emit(IR::PushNull);
        }
        
        // Store the value in the variable
        self.emit(IR::StoreVar(name));
    }
    
    fn compile_function_declaration(&mut self, name: String, parameters: Vec<String>, body: Vec<Statement>) {
        // Save the current function name
        let old_function = self.current_function.clone();
        self.current_function = Some(name.clone());
        
        // Generate a unique label for the function
        let function_label = self.generate_label("function_");
        let end_label = self.generate_label("end_");
        
        // Emit a jump to skip over the function body
        let jump_pos = self.emit(IR::Jump(0));
        
        // Mark the start of the function
        let function_start = self.emit_label(&function_label);
        
        // Register the function in the function table
        self.function_table.define(&name, function_start);
        
        // Create a new scope for the function body
        self.enter_scope();
        
        // Define parameters in the function's scope
        for param in parameters {
            self.symbol_table.define(&param);
        }
        
        // Compile the function body
        for stmt in body {
            self.compile_statement(stmt);
        }
        
        // Ensure the function returns (even if there's no explicit return)
        self.emit(IR::PushNull);
        self.emit(IR::Return);
        
        // Mark the end of the function
        let function_end = self.emit_label(&end_label);
        
        // Update the jump instruction to skip over the function body
        self.replace_instruction(jump_pos, IR::Jump(function_end));
        
        // Leave the function's scope
        self.leave_scope();
        
        // Restore the previous function name
        self.current_function = old_function;
    }
    
    fn compile_return_statement(&mut self, value: Option<Expression>) {
        // Compile the return value if it exists
        if let Some(expr) = value {
            self.compile_expression(expr);
        } else {
            // If no return value, push null
            self.emit(IR::PushNull);
        }
        
        // Emit the return instruction
        self.emit(IR::Return);
    }
    
    fn compile_block_statement(&mut self, statements: Vec<Statement>) {
        // Create a new scope for the block
        self.enter_scope();
        
        // Compile each statement in the block
        for stmt in statements {
            self.compile_statement(stmt);
        }
        
        // Leave the block's scope
        self.leave_scope();
    }
    
    fn compile_if_statement(&mut self, condition: Expression, consequence: Vec<Statement>, alternative: Option<Vec<Statement>>) {
        // Generate labels for the if statement
        let else_label = self.generate_label("else_");
        let end_label = self.generate_label("end_");
        
        // Compile the condition
        self.compile_expression(condition);
        
        // Emit a conditional jump to the else branch
        self.emit(IR::JumpIfFalse(0)); // Placeholder for else_label
        let jump_to_else_pos = self.ir.len() - 1;
        
        // Compile the consequence (if branch)
        for stmt in consequence {
            self.compile_statement(stmt);
        }
        
        // Emit a jump to the end of the if statement
        self.emit(IR::Jump(0)); // Placeholder for end_label
        let jump_to_end_pos = self.ir.len() - 1;
        
        // Mark the start of the else branch
        let else_pos = self.emit_label(&else_label);
        
        // Update the jump to else instruction
        self.replace_instruction(jump_to_else_pos, IR::JumpIfFalse(else_pos));
        
        // Compile the alternative (else branch) if it exists
        if let Some(alt) = alternative {
            for stmt in alt {
                self.compile_statement(stmt);
            }
        }
        
        // Mark the end of the if statement
        let end_pos = self.emit_label(&end_label);
        
        // Update the jump to end instruction
        self.replace_instruction(jump_to_end_pos, IR::Jump(end_pos));
    }
    
    fn compile_while_statement(&mut self, condition: Expression, body: Vec<Statement>) {
        // Generate labels for the while loop
        let loop_label = self.generate_label("loop_");
        let end_label = self.generate_label("end_");
        
        // Mark the start of the loop
        let loop_start = self.emit_label(&loop_label);
        
        // Enter the loop context for break/continue statements
        self.enter_loop();
        
        // Compile the condition
        self.compile_expression(condition);
        
        // Emit a conditional jump to the end of the loop
        self.emit(IR::JumpIfFalse(0)); // Placeholder for end_label
        let jump_to_end_pos = self.ir.len() - 1;
        
        // Compile the loop body
        for stmt in body {
            self.compile_statement(stmt);
        }
        
        // Emit a jump back to the start of the loop
        self.emit(IR::Jump(loop_start));
        
        // Mark the end of the loop
        let end_pos = self.emit_label(&end_label);
        
        // Update the jump to end instruction
        self.replace_instruction(jump_to_end_pos, IR::JumpIfFalse(end_pos));
        
        // Leave the loop context and patch break/continue statements
        self.leave_loop(loop_start, end_pos);
    }
    
    fn compile_for_statement(&mut self, iterator: String, iterable: Expression, body: Vec<Statement>) {
        // Generate labels for the for loop
        let loop_label = self.generate_label("for_loop_");
        let end_label = self.generate_label("for_end_");
        
        // Create a new scope for the loop
        self.enter_scope();
        
        // Define the iterator variable
        self.symbol_table.define(&iterator);
        
        // Compile the iterable expression
        self.compile_expression(iterable);
        
        // Create a temporary index variable
        let index_var = format!("__index_{}", self.generate_label(""));
        self.symbol_table.define(&index_var);
        
        // Initialize the index to 0
        self.emit(IR::PushNumber(0.0));
        self.emit(IR::StoreVar(index_var.clone()));
        
        // Mark the start of the loop
        let loop_start = self.emit_label(&loop_label);
        
        // Enter the loop context for break/continue statements
        self.enter_loop();
        
        // Check if the index is less than the length of the iterable
        self.emit(IR::LoadVar(index_var.clone()));
        self.emit(IR::Dup); // Duplicate the index for the array access
        self.emit(IR::GetIndex); // Get the element at the current index
        
        // Store the current element in the iterator variable
        self.emit(IR::StoreVar(iterator));
        
        // Increment the index
        self.emit(IR::LoadVar(index_var.clone()));
        self.emit(IR::PushNumber(1.0));
        self.emit(IR::Add);
        self.emit(IR::StoreVar(index_var.clone()));
        
        // Compile the loop body
        for stmt in body {
            self.compile_statement(stmt);
        }
        
        // Emit a jump back to the start of the loop
        self.emit(IR::Jump(loop_start));
        
        // Mark the end of the loop
        let end_pos = self.emit_label(&end_label);
        
        // Leave the loop context and patch break/continue statements
        self.leave_loop(loop_start, end_pos);
        
        // Leave the loop's scope
        self.leave_scope();
    }
    
    fn compile_break_statement(&mut self) {
        // Check if we're inside a loop
        if self.break_stack.is_empty() {
            panic!("Break statement outside of loop");
        }
        
        // Emit a jump to the end of the loop (will be patched later)
        let break_pos = self.emit(IR::Jump(0)); // Placeholder
        
        // Add the break position to the current loop's break stack
        if let Some(breaks) = self.break_stack.last_mut() {
            breaks.push(break_pos);
        }
    }
    
    fn compile_continue_statement(&mut self) {
        // Check if we're inside a loop
        if self.continue_stack.is_empty() {
            panic!("Continue statement outside of loop");
        }
        
        // Emit a jump to the start of the loop (will be patched later)
        let continue_pos = self.emit(IR::Jump(0)); // Placeholder
        
        // Add the continue position to the current loop's continue stack
        if let Some(continues) = self.continue_stack.last_mut() {
            continues.push(continue_pos);
        }
    }
    
    fn compile_show_statement(&mut self, value: Expression) {
        // Compile the value to be shown
        self.compile_expression(value);
        
        // Emit the print instruction
        self.emit(IR::Print);
    }
    
    fn compile_try_statement(&mut self, try_block: Vec<Statement>, catch_block: Option<Vec<Statement>>, finally_block: Option<Vec<Statement>>) {
        // This is a simplified implementation of try/catch/finally
        // In a real implementation, this would involve exception handling
        
        // Compile the try block
        for stmt in try_block {
            self.compile_statement(stmt);
        }
        
        // Compile the catch block if it exists
        if let Some(catch) = catch_block {
            for stmt in catch {
                self.compile_statement(stmt);
            }
        }
        
        // Compile the finally block if it exists
        if let Some(finally) = finally_block {
            for stmt in finally {
                self.compile_statement(stmt);
            }
        }
    }
    
    fn compile_throw_statement(&mut self, value: Expression) {
        // Compile the value to be thrown
        self.compile_expression(value);
        
        // In a real implementation, this would involve exception handling
        // For now, we'll just print the error and exit
        self.emit(IR::Print);
    }
    
    fn compile_read_statement(&mut self, name: String) {
        // Define the variable in the symbol table
        self.symbol_table.define(&name);
        
        // Add a custom IR operation for reading user input
        self.emit(IR::ReadInput);
        self.emit(IR::StoreVar(name));
    }
    
    fn compile_exit_statement(&mut self) {
        // Emit the exit instruction to terminate the program
        self.emit(IR::Exit);
    }
    
    fn compile_document_type_declaration(&mut self, doc_type: String) {
        // Store the document type in the compiler state
        // This doesn't generate any runtime code as it's a compile-time directive
        if !self.clean_output {
            println!("Document type set to: {}", doc_type);
        }
    }
    
    fn compile_expression(&mut self, expr: Expression) {
        match expr {
            Expression::Identifier(name) => {
                self.emit(IR::LoadVar(name));
            },
            Expression::StringLiteral(value) => {
                self.emit(IR::PushString(value));
            },
            Expression::NumberLiteral(value) => {
                self.emit(IR::PushNumber(value));
            },
            Expression::BooleanLiteral(value) => {
                self.emit(IR::PushBoolean(value));
            },
            Expression::NullLiteral => {
                self.emit(IR::PushNull);
            },
            Expression::PrefixExpression { operator, right } => {
                self.compile_prefix_expression(operator, *right);
            },
            Expression::InfixExpression { left, operator, right } => {
                self.compile_infix_expression(*left, operator, *right);
            },
            Expression::AssignmentExpression { left, operator, right } => {
                self.compile_assignment_expression(*left, operator, *right);
            },
            Expression::CallExpression { function, arguments } => {
                self.compile_call_expression(*function, arguments);
            },
            Expression::ArrayLiteral { elements } => {
                self.compile_array_literal(elements);
            },
            Expression::IndexExpression { left, index } => {
                self.compile_index_expression(*left, *index);
            },
            Expression::MapLiteral { pairs } => {
                self.compile_map_literal(pairs);
            },
        }
    }
    
    fn compile_prefix_expression(&mut self, operator: String, right: Expression) {
        // Compile the right operand
        self.compile_expression(right);
        
        // Apply the prefix operator
        match operator.as_str() {
            "-" => { self.emit(IR::Negate); },
            "!" => { self.emit(IR::Not); },
            _ => panic!("Unknown prefix operator: {}", operator),
        }
    }
    
    fn compile_infix_expression(&mut self, left: Expression, operator: String, right: Expression) {
        // Compile the left operand
        self.compile_expression(left);
        
        // Compile the right operand
        self.compile_expression(right);
        
        // Apply the infix operator
        match operator.as_str() {
            "+" => { self.emit(IR::Add); },
            "-" => { self.emit(IR::Subtract); },
            "*" => { self.emit(IR::Multiply); },
            "/" => { self.emit(IR::Divide); },
            "%" => { self.emit(IR::Modulo); },
            "**" => { self.emit(IR::Power); },
            "//" => { self.emit(IR::FloorDiv); },
            "==" => { self.emit(IR::Equal); },
            "!=" => { self.emit(IR::NotEqual); },
            ">" => { self.emit(IR::GreaterThan); },
            ">=" => { self.emit(IR::GreaterEqual); },
            "<" => { self.emit(IR::LessThan); },
            "<=" => { self.emit(IR::LessEqual); },
            "&&" => { self.emit(IR::And); },
            "||" => { self.emit(IR::Or); },
            _ => panic!("Unknown infix operator: {}", operator),
        }
    }
    
    fn compile_assignment_expression(&mut self, left: Expression, operator: String, right: Expression) {
        // For simple assignment (=), just compile the right expression and store it
        if operator == "=" {
            // Compile the right expression
            self.compile_expression(right);
            
            // Store the result in the variable
            if let Expression::Identifier(name) = left {
                self.emit(IR::StoreVar(name));
            } else if let Expression::IndexExpression { left: array, index } = left {
                // Compile the array and index
                self.compile_expression(*array);
                self.compile_expression(*index);
                
                // Store the value in the array at the given index
                self.emit(IR::SetIndex);
            } else {
                panic!("Invalid left-hand side in assignment");
            }
        } else {
            // For compound assignments (+=, -=, etc.), load the variable, apply the operation, and store it back
            if let Expression::Identifier(name) = left.clone() {
                // Load the current value
                self.emit(IR::LoadVar(name.clone()));
                
                // Compile the right expression
                self.compile_expression(right);
                
                // Apply the operation
                match operator.as_str() {
                    "+=" => { self.emit(IR::Add); },
                    "-=" => { self.emit(IR::Subtract); },
                    "*=" => { self.emit(IR::Multiply); },
                    "/=" => { self.emit(IR::Divide); },
                    "%=" => { self.emit(IR::Modulo); },
                    _ => panic!("Unknown assignment operator: {}", operator),
                }
                
                // Store the result back in the variable
                self.emit(IR::StoreVar(name));
            } else {
                panic!("Invalid left-hand side in compound assignment");
            }
        }
    }
    
    fn compile_call_expression(&mut self, function: Expression, arguments: Vec<Expression>) {
        // Compile each argument
        for arg in &arguments {
            self.compile_expression(arg.clone());
        }
        
        // Get the function name
        if let Expression::Identifier(name) = function {
            // Call the function with the given number of arguments
            self.emit(IR::Call(name, arguments.len()));
        } else {
            panic!("Function call on non-identifier");
        }
    }
    
    fn compile_array_literal(&mut self, elements: Vec<Expression>) {
        // Compile each element
        for elem in &elements {
            self.compile_expression(elem.clone());
        }
        
        // Create the array with the given number of elements
        self.emit(IR::CreateArray(elements.len()));
    }
    
    fn compile_index_expression(&mut self, left: Expression, index: Expression) {
        // Compile the array
        self.compile_expression(left);
        
        // Compile the index
        self.compile_expression(index);
        
        // Get the element at the given index
        self.emit(IR::GetIndex);
    }
    
    fn compile_map_literal(&mut self, pairs: Vec<(Expression, Expression)>) {
        // Compile each key-value pair
        for (key, value) in &pairs {
            self.compile_expression(key.clone());
            self.compile_expression(value.clone());
        }
        
        // Create the map with the given number of key-value pairs
        self.emit(IR::CreateMap(pairs.len()));
    }
    
    // Generate machine code from the IR
    pub fn generate_machine_code(&self) -> Result<Vec<u8>, String> {
        // This is a simplified implementation that would be replaced with actual machine code generation
        // In a real implementation, this would translate the IR to native machine code
        
        // For now, we'll just return a placeholder binary
        let mut code = Vec::new();
        
        // Add a simple header
        code.extend_from_slice(b"\x7FELF"); // ELF magic number
        
        // Add placeholder code for each IR instruction
        for ir in &self.ir {
            match ir {
                IR::PushNumber(_) => code.push(0x01),
                IR::PushString(_) => code.push(0x02),
                IR::PushBoolean(_) => code.push(0x03),
                IR::PushNull => code.push(0x04),
                IR::Pop => code.push(0x05),
                IR::Dup => code.push(0x06),
                IR::Swap => code.push(0x07),
                IR::StoreVar(_) => code.push(0x08),
                IR::LoadVar(_) => code.push(0x09),
                IR::Add => code.push(0x0A),
                IR::Subtract => code.push(0x0B),
                IR::Multiply => code.push(0x0C),
                IR::Divide => code.push(0x0D),
                IR::Modulo => code.push(0x0E),
                IR::Power => code.push(0x0F),
                IR::FloorDiv => code.push(0x10),
                IR::Negate => code.push(0x11),
                IR::Equal => code.push(0x12),
                IR::NotEqual => code.push(0x13),
                IR::GreaterThan => code.push(0x14),
                IR::GreaterEqual => code.push(0x15),
                IR::LessThan => code.push(0x16),
                IR::LessEqual => code.push(0x17),
                IR::And => code.push(0x18),
                IR::Or => code.push(0x19),
                IR::Not => code.push(0x1A),
                IR::Jump(_) => code.push(0x1B),
                IR::JumpIfFalse(_) => code.push(0x1C),
                IR::JumpIfTrue(_) => code.push(0x1D),
                IR::Call(_, _) => code.push(0x1E),
                IR::Return => code.push(0x1F),
                IR::Print => code.push(0x20),
                IR::ReadInput => code.push(0x29),
                IR::Exit => code.push(0x2A),
                IR::CreateArray(_) => code.push(0x21),
                IR::GetIndex => code.push(0x22),
                IR::SetIndex => code.push(0x23),
                IR::CreateMap(_) => code.push(0x24),
                IR::GetKey => code.push(0x25),
                IR::SetKey => code.push(0x26),
                IR::DefineFunction(_, _) => code.push(0x27),
                IR::Label(_) => code.push(0x28),
                IR::SetGlobal(_) => code.push(0x2B),  // Add SetGlobal instruction
            }
        }
        
        Ok(code)
    }
    
    // Write the generated machine code to a file
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        match self.generate_machine_code() {
            Ok(code) => {
                match fs::write(path, code) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Failed to write to file: {}", e)),
                }
            },
            Err(e) => Err(e),
        }
    }
    
    // Execute the compiled code directly
    pub fn execute(&self) -> Result<(), String> {
        // In a real implementation, this would execute the generated machine code
        // For now, we'll just interpret the IR directly
        
        // Only show debug output if not in clean output mode
        if !self.clean_output {
            println!("Executing Razen program...");
            
            // Print the IR for debugging
            for (i, ir) in self.ir.iter().enumerate() {
                println!("{}: {:?}", i, ir);
            }
        }
        
        // Simple stack-based interpreter for the IR
        let mut stack: Vec<String> = Vec::new();
        let mut variables: HashMap<String, String> = HashMap::new();
        
        // Execute each instruction
        let mut pc = 0; // Program counter
        while pc < self.ir.len() {
            let ir = &self.ir[pc];
            match ir {
                IR::PushNumber(n) => stack.push(n.to_string()),
                IR::PushString(s) => stack.push(s.clone()),
                IR::PushBoolean(b) => stack.push(b.to_string()),
                IR::PushNull => stack.push("null".to_string()),
                IR::Pop => {
                    if let Some(_) = stack.pop() {
                        // Do nothing
                    }
                },
                IR::Dup => {
                    if let Some(value) = stack.last().cloned() {
                        stack.push(value);
                    }
                },
                IR::Swap => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        stack.push(b);
                        stack.push(a);
                    }
                },
                IR::StoreVar(name) => {
                    if let Some(value) = stack.pop() {
                        variables.insert(name.clone(), value);
                    }
                },
                IR::LoadVar(name) => {
                    if let Some(value) = variables.get(name) {
                        stack.push(value.clone());
                    } else {
                        stack.push("undefined".to_string());
                    }
                },
                IR::SetGlobal(name) => {
                    if let Some(value) = stack.pop() {
                        variables.insert(name.clone(), value);
                    }
                },
                IR::Add => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        // Try to add as numbers first
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            stack.push((a_num + b_num).to_string());
                        } else {
                            // Otherwise concatenate as strings
                            stack.push(format!("{}{}", a, b));
                        }
                    }
                },
                IR::Subtract => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            stack.push((a_num - b_num).to_string());
                        }
                    }
                },
                IR::Multiply => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            stack.push((a_num * b_num).to_string());
                        }
                    }
                },
                IR::Divide => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            if b_num != 0.0 {
                                stack.push((a_num / b_num).to_string());
                            } else {
                                return Err("Division by zero".to_string());
                            }
                        }
                    }
                },
                IR::Equal => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        // Compare as strings
                        let result = a == b;
                        stack.push(result.to_string());
                    }
                },
                IR::NotEqual => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        // Compare as strings
                        let result = a != b;
                        stack.push(result.to_string());
                    }
                },
                IR::GreaterThan => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        // Try to compare as numbers first
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            stack.push((a_num > b_num).to_string());
                        } else {
                            // Otherwise compare as strings
                            let result = a > b;
                            stack.push(result.to_string());
                        }
                    }
                },
                IR::GreaterEqual => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        // Try to compare as numbers first
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            stack.push((a_num >= b_num).to_string());
                        } else {
                            // Otherwise compare as strings
                            let result = a >= b;
                            stack.push(result.to_string());
                        }
                    }
                },
                IR::LessThan => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        // Try to compare as numbers first
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            stack.push((a_num < b_num).to_string());
                        } else {
                            // Otherwise compare as strings
                            let result = a < b;
                            stack.push(result.to_string());
                        }
                    }
                },
                IR::LessEqual => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        // Try to compare as numbers first
                        if let (Ok(a_num), Ok(b_num)) = (a.parse::<f64>(), b.parse::<f64>()) {
                            stack.push((a_num <= b_num).to_string());
                        } else {
                            // Otherwise compare as strings
                            let result = a <= b;
                            stack.push(result.to_string());
                        }
                    }
                },
                IR::Swap => {
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        stack.push(b);
                        stack.push(a);
                    }
                },
                IR::JumpIfFalse(target) => {
                    if let Some(value) = stack.pop() {
                        if value == "false" || value == "0" || value.is_empty() || value == "False" {
                            pc = *target;
                            continue;
                        }
                    }
                },
                IR::JumpIfTrue(target) => {
                    if let Some(value) = stack.pop() {
                        if value == "true" || value == "1" || value == "True" {
                            pc = *target;
                            continue;
                        }
                    }
                },
                IR::GetKey => {
                    if let (Some(key), Some(map_str)) = (stack.pop(), stack.pop()) {
                        // Simple implementation: assume map is stored as a string in format "key1:value1,key2:value2"
                        let map_entries: Vec<&str> = map_str.split(',').collect();
                        let mut found = false;
                        
                        for entry in map_entries {
                            let kv: Vec<&str> = entry.split(':').collect();
                            if kv.len() == 2 && kv[0] == key {
                                stack.push(kv[1].to_string());
                                found = true;
                                break;
                            }
                        }
                        
                        if !found {
                            stack.push("undefined".to_string());
                        }
                    }
                },
                IR::SetKey => {
                    if let (Some(value), Some(key), Some(map_str)) = (stack.pop(), stack.pop(), stack.pop()) {
                        // Simple implementation: assume map is stored as a string in format "key1:value1,key2:value2"
                        let map_entries: Vec<&str> = map_str.split(',').collect();
                        let mut new_map = Vec::new();
                        let mut found = false;
                        
                        for entry in map_entries {
                            if entry.is_empty() {
                                continue;
                            }
                            
                            let kv: Vec<&str> = entry.split(':').collect();
                            if kv.len() == 2 && kv[0] == key {
                                new_map.push(format!("{key}:{value}"));
                                found = true;
                            } else {
                                new_map.push(entry.to_string());
                            }
                        }
                        
                        if !found {
                            new_map.push(format!("{key}:{value}"));
                        }
                        
                        stack.push(new_map.join(","));
                    }
                },
                IR::DefineFunction(name, address) => {
                    // Store function definition in variables map
                    variables.insert(format!("__func_{}", name), address.to_string());
                },
                IR::Return => {
                    // The return value is already on the stack from the previous expression
                    // We need to make sure it stays on the stack and is used as the function result
                    // For debugging purposes, we'll print the return value
                    if !self.clean_output {
                        if let Some(value) = stack.last() {
                            println!("Returning value: {}", value);
                        }
                    }
                    // Skip to the end of the function or to a specific return point
                    // For now, we'll just continue execution and rely on the value being on the stack
                },
                IR::Call(name, arg_count) => {
                    // Function call implementation
                    if !self.clean_output {
                        println!("Calling function: {} with {} arguments", name, arg_count);
                    }
                    
                    // Pop the arguments from the stack (in reverse order)
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        if let Some(arg) = stack.pop() {
                            args.push(arg);
                        }
                    }
                    args.reverse(); // Reverse to get the original order
                    
                    // For debugging
                    if !self.clean_output {
                        for (i, arg) in args.iter().enumerate() {
                            println!("Arg {}: {}", i, arg);
                        }
                    }
                    
                    // Process the function call based on the function name
                    // For now, we'll just push the result back onto the stack
                    // In a real implementation, we would look up the function and execute it
                    
                    // For built-in functions, we can implement them directly here
                    match name.as_str() {
                        "add" => {
                            if args.len() >= 2 {
                                if let (Ok(a), Ok(b)) = (args[0].parse::<f64>(), args[1].parse::<f64>()) {
                                    stack.push((a + b).to_string());
                                }
                            }
                        },
                        "subtract" => {
                            if args.len() >= 2 {
                                if let (Ok(a), Ok(b)) = (args[0].parse::<f64>(), args[1].parse::<f64>()) {
                                    stack.push((a - b).to_string());
                                }
                            }
                        },
                        "multiply" => {
                            if args.len() >= 2 {
                                if let (Ok(a), Ok(b)) = (args[0].parse::<f64>(), args[1].parse::<f64>()) {
                                    stack.push((a * b).to_string());
                                }
                            }
                        },
                        "divide" => {
                            if args.len() >= 2 {
                                if let (Ok(a), Ok(b)) = (args[0].parse::<f64>(), args[1].parse::<f64>()) {
                                    if b != 0.0 {
                                        stack.push((a / b).to_string());
                                    } else {
                                        stack.push("Error: Division by zero".to_string());
                                    }
                                }
                            }
                        },
                        _ => {
                            // For user-defined functions, we would look them up and execute them
                            // For now, we'll just return a placeholder value
                            if !self.clean_output {
                                println!("Unknown function: {}", name);
                            }
                            // Push the last argument as the result (for testing)
                            if !args.is_empty() {
                                stack.push(args.last().unwrap().clone());
                            } else {
                                stack.push("undefined".to_string());
                            }
                        }
                    }
                },
                // Add basic implementations for other instructions as needed
                _ => {
                    // For instructions not yet implemented, just log if in debug mode
                    if !self.clean_output {
                        println!("Unimplemented instruction: {:?}", ir);
                    }
                }
            }
            pc += 1; // Move to next instruction
        }
        
        if !self.clean_output {
            println!("Execution complete.");
        }
        
        Ok(())
    }
    
    // Module System Methods
    
    /// Compile module import statement
    fn compile_module_import(&mut self, names: Vec<String>, alias: Option<String>, source: String) {
        if !self.clean_output {
            println!("[Compiler] Importing module: {} from {}", names.join(", "), source);
        }
        
        // Load the module file
        let module_path = source.trim_matches('"');
        
        // Check if we need to add .rzn extension
        let module_file = if module_path.ends_with(".rzn") {
            module_path.to_string()
        } else {
            format!("{}.rzn", module_path)
        };
        
        // Try to find the module in standard library first, then relative to current file
        let module_content = match fs::read_to_string(&module_file) {
            Ok(content) => content,
            Err(_) => {
                // Try standard library path
                let std_lib_path = format!("stdlib/{}", module_file);
                match fs::read_to_string(&std_lib_path) {
                    Ok(content) => content,
                    Err(_) => {
                        self.errors.push(format!("Module not found: {}", module_file));
                        return;
                    }
                }
            }
        };
        
        // Parse the module
        let mut lexer = crate::lexer::Lexer::new(module_content);
        let mut parser = crate::parser::Parser::new(lexer);
        let module_program = parser.parse_program();
        
        // Check for parser errors
        if !parser.get_errors().is_empty() {
            for error in parser.get_errors() {
                self.errors.push(format!("Error parsing module {}: {}", module_file, error));
            }
            return;
        }
        
        // Create a new compiler for the module
        let mut module_compiler = Compiler::new();
        module_compiler.set_clean_output(true);
        
        // Compile the module
        module_compiler.compile_program(module_program);
        
        // Import the exported symbols from the module
        if let Some(alias_name) = alias {
            // Import as namespace
            self.emit(IR::PushString(format!("Importing module {} as {}", source, alias_name)));
            self.emit(IR::Call("__import_module".to_string(), 1));
        } else {
            // Import specific names
            for name in names {
                // Define the symbol in current scope
                self.symbol_table.define(&name);
                
                // For constants like PI, we need to initialize them
                if name == "PI" {
                    self.emit(IR::PushNumber(3.14159265359));
                    self.emit(IR::SetGlobal(name.clone()));
                } else if name == "E" {
                    self.emit(IR::PushNumber(2.71828182846));
                    self.emit(IR::SetGlobal(name.clone()));
                } else {
                    // For functions, just import the symbol
                    self.emit(IR::PushString(format!("Importing {} from {}", name, source)));
                    self.emit(IR::Call("__import_symbol".to_string(), 1));
                }
            }
        }
    }
    
    /// Compile module export statement
    fn compile_module_export(&mut self, name: String) {
        if !self.clean_output {
            println!("[Compiler] Exporting symbol: {}", name);
        }
        
        // Check if the symbol exists in current scope
        if self.symbol_table.resolve(&name).is_none() && self.function_table.resolve(&name).is_none() {
            self.errors.push(format!("Cannot export undefined symbol: {}", name));
            return;
        }
        
        // Mark the symbol as exported
        self.emit(IR::PushString(name));
        self.emit(IR::Call("__export_symbol".to_string(), 1));
    }
    
    // Developer Tools Methods
    
    /// Compile debug statement
    fn compile_debug_statement(&mut self, value: Expression) {
        if !self.clean_output {
            println!("[Compiler] Debug statement");
        }
        
        // Compile the expression to debug
        self.compile_expression(value);
        
        // Call debug function
        self.emit(IR::Call("__debug".to_string(), 1));
    }
    
    /// Compile assert statement
    fn compile_assert_statement(&mut self, condition: Expression, message: Option<Expression>) {
        if !self.clean_output {
            println!("[Compiler] Assert statement");
        }
        
        // Compile the condition
        self.compile_expression(condition);
        
        // If there's a message, compile it too
        if let Some(msg) = message {
            self.compile_expression(msg);
            self.emit(IR::Call("__assert_with_message".to_string(), 2));
        } else {
            self.emit(IR::Call("__assert".to_string(), 1));
        }
    }
    
    /// Compile trace statement
    fn compile_trace_statement(&mut self, value: Expression) {
        if !self.clean_output {
            println!("[Compiler] Trace statement");
        }
        
        // Compile the expression to trace
        self.compile_expression(value);
        
        // Call trace function
        self.emit(IR::Call("__trace".to_string(), 1));
    }
}