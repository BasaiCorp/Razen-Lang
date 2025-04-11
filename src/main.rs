mod token;
mod ast;
mod lexer;
mod parser;
mod compiler;
mod syntax;

use std::env;
use std::path::Path;
use std::process;
use std::fs;
use std::io;
use std::time::Instant;

fn print_usage() {
    println!("Usage: razen <command> [args]\n");
    println!("Commands:");
    println!("  compile <file>     Compile a Razen source file to machine code");
    println!("  run <file>         Compile and execute a Razen source file");
    println!("  test [dir|file]    Run tests in the specified directory or file");
    println!("  help               Display this help message");
    println!("\nOptions:");
    println!("  --debug            Enable debug mode with additional output");
    println!("  --clean-output     Only show program output (no IR or debug info)");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }
    
    // Check for debug flag
    let debug_mode = args.iter().any(|arg| arg == "--debug");
    
    // Check for clean output flag (used by razen-run to only show program output)
    let clean_output = args.iter().any(|arg| arg == "--clean-output");
    
    // Filter out the special flags from arguments
    let filtered_args: Vec<String> = args.iter()
        .filter(|&arg| arg != "--debug" && arg != "--clean-output")
        .cloned()
        .collect();
    
    match filtered_args[1].as_str() {
        "compile" => {
            if filtered_args.len() < 3 {
                println!("Error: Missing source file path");
                process::exit(1);
            }
            
            let source_path = &filtered_args[2];
            let output_path = if filtered_args.len() > 3 {
                &filtered_args[3]
            } else {
                // Default output path: replace .rzn extension with .bin
                let source_path_obj = Path::new(source_path);
                let stem = source_path_obj.file_stem().unwrap_or_default().to_str().unwrap_or("output");
                &format!("{}.bin", stem)
            };
            
            println!("Compiling {} to {}", source_path, output_path);
            
            match compiler::Compiler::from_file(source_path) {
                Ok(compiler) => {
                    match compiler.write_to_file(output_path) {
                        Ok(_) => println!("Compilation successful!"),
                        Err(e) => {
                            println!("\n🔴 Compiler Error: {}", e);
                            println!("\n💡 Tip: Check for type mismatches or undefined variables in your code.");
                            process::exit(1);
                        }
                    }
                },
                Err(e) => {
                    println!("\n🔴 Compilation Error: {}", e);
                    println!("\n💡 Tip: Check the syntax and variable names in your code. Make sure you're using the correct keywords and variable types.");
                    process::exit(1);
                }
            }
        },
        "run" => {
            if filtered_args.len() < 3 {
                println!("Error: Missing source file path");
                process::exit(1);
            }
            
            let source_path = &filtered_args[2];
            
            if !clean_output {
                println!("Running {}", source_path);
                
                if debug_mode {
                    println!("Debug mode enabled");
                }
            }
            
            match compiler::Compiler::from_file(source_path) {
                Ok(compiler) => {
                    match compiler.execute() {
                        Ok(_) => {
                            if !clean_output {
                                println!("\n✅ Program executed successfully");
                            }
                        },
                        Err(e) => {
                            println!("\n🔴 Runtime Error: {}", e);
                            println!("\n💡 Tip: Check for logical errors or invalid operations in your code.");
                            process::exit(1);
                        }
                    }
                },
                Err(e) => {
                    println!("\n🔴 Compilation Error: {}", e);
                    println!("\n💡 Tip: Check the syntax and variable names in your code. Make sure you're using the correct keywords and variable types.");
                    process::exit(1);
                }
            }
        },
        "test" => {
            println!("Running tests");
            
            if debug_mode {
                println!("Debug mode enabled");
            }
            
            let test_path = if filtered_args.len() > 2 {
                &filtered_args[2]
            } else {
                // Default to razen_tests directory
                "razen_tests"
            };
            
            println!("Test path: {}", test_path);
            
            // Check if path is a directory or file
            let path = Path::new(test_path);
            if path.is_dir() {
                // Run all tests in directory
                println!("Running all tests in directory: {}", test_path);
                run_tests_in_directory(path, debug_mode);
            } else if path.is_file() {
                // Run single test file
                println!("Running test file: {}", test_path);
                run_test_file(path, debug_mode);
            } else {
                println!("Error: Test path '{}' does not exist", test_path);
                process::exit(1);
            }
        },
        "help" | "-h" | "--help" => {
            print_usage();
        },
        _ => {
            println!("Unknown command: {}", filtered_args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

// Run all tests in a directory
fn run_tests_in_directory(dir_path: &Path, debug_mode: bool) -> io::Result<()> {
    let mut passed = 0;
    let mut failed = 0;
    let mut total_time = 0.0;
    
    // Collect all .rzn files in the directory
    let entries = fs::read_dir(dir_path)?;
    let mut test_files = Vec::new();
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "rzn") {
            test_files.push(path);
        } else if path.is_dir() {
            // Recursively run tests in subdirectories
            if debug_mode {
                println!("Entering directory: {}", path.display());
            }
            run_tests_in_directory(&path, debug_mode)?;
        }
    }
    
    // Sort test files for consistent output
    test_files.sort();
    
    println!("Found {} test files in {}", test_files.len(), dir_path.display());
    
    for test_file in test_files {
        let (success, duration) = run_test_file(&test_file, debug_mode);
        
        if success {
            passed += 1;
        } else {
            failed += 1;
        }
        
        total_time += duration;
    }
    
    println!("\nTest Summary for {}:", dir_path.display());
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);
    println!("  Total: {}", passed + failed);
    println!("  Time: {:.2}s", total_time);
    
    Ok(())
}

// Run a single test file
fn run_test_file(file_path: &Path, debug_mode: bool) -> (bool, f64) {
    let start = Instant::now();
    let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
    
    print!("Testing {}... ", file_name);
    
    if debug_mode {
        println!();
    }
    
    match compiler::Compiler::from_file(file_path.to_str().unwrap()) {
        Ok(compiler) => {
            match compiler.execute() {
                Ok(_) => {
                    let duration = start.elapsed().as_secs_f64();
                    if !debug_mode {
                        println!("PASS ({:.2}s)", duration);
                    } else {
                        println!("\n✅ PASS ({:.2}s)", duration);
                    }
                    (true, duration)
                },
                Err(e) => {
                    let duration = start.elapsed().as_secs_f64();
                    if !debug_mode {
                        println!("FAIL ({:.2}s)", duration);
                    } else {
                        println!("\n🔴 FAIL ({:.2}s): {}", duration, e);
                        println!("\n💡 Tip: Check for logical errors or invalid operations in your code.");
                    }
                    (false, duration)
                }
            }
        },
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            if !debug_mode {
                println!("FAIL ({:.2}s)", duration);
            } else {
                println!("\n🔴 FAIL ({:.2}s): {}", duration, e);
                println!("\n💡 Tip: Check the syntax and variable names in your code. Make sure you're using the correct keywords and variable types.");
            }
            (false, duration)
        }
    }
}
