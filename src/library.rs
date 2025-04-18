use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::value::Value;

/// LibraryFunction represents a callable function in a library
pub type LibraryFunction = fn(Vec<Value>) -> Result<Value, String>;

/// Library represents a collection of functions
#[derive(Clone)]
pub struct Library {
    name: String,
    functions: HashMap<String, LibraryFunction>,
}

impl Library {
    /// Create a new library with the given name
    pub fn new(name: &str) -> Self {
        Library {
            name: name.to_string(),
            functions: HashMap::new(),
        }
    }

    /// Register a function in the library
    pub fn register_function(&mut self, name: &str, function: LibraryFunction) {
        self.functions.insert(name.to_string(), function);
    }

    /// Call a function in the library
    pub fn call_function(&self, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
        match self.functions.get(function_name) {
            Some(function) => function(args),
            None => Err(format!("Function '{}' not found in library '{}'", function_name, self.name)),
        }
    }

    /// Get the name of the library
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if a function exists in the library
    pub fn has_function(&self, function_name: &str) -> bool {
        self.functions.contains_key(function_name)
    }

    /// Get all function names in the library
    pub fn function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }
}

impl fmt::Debug for Library {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Library {{ name: {}, functions: {:?} }}", self.name, self.function_names())
    }
}

/// LibraryManager manages all libraries in the system
#[derive(Debug, Clone)]
pub struct LibraryManager {
    libraries: HashMap<String, Library>,
}

impl LibraryManager {
    /// Create a new library manager
    pub fn new() -> Self {
        LibraryManager {
            libraries: HashMap::new(),
        }
    }

    /// Register a library
    pub fn register_library(&mut self, library: Library) {
        let name = library.name().to_string();
        self.libraries.insert(name.to_lowercase(), library);
    }

    /// Get a library by name (case-insensitive)
    pub fn get_library(&self, name: &str) -> Option<&Library> {
        self.libraries.get(&name.to_lowercase())
    }

    /// Call a library function
    pub fn call_library(&self, library_name: &str, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Handle case-insensitive library names
        let library_name = library_name.to_lowercase();
        
        // Support both PascalCase and lowercase for library names
        match self.libraries.get(&library_name) {
            Some(library) => library.call_function(function_name, args),
            None => Err(format!("Library '{}' not found", library_name)),
        }
    }

    /// Initialize all standard libraries
    pub fn initialize_standard_libraries(&mut self) {
        self.register_standard_libraries();
    }

    /// Register all standard libraries
    fn register_standard_libraries(&mut self) {
        // Array library
        let mut arr_lib = Library::new("arrlib");
        arr_lib.register_function("push", crate::functions::arrlib::push);
        arr_lib.register_function("pop", crate::functions::arrlib::pop);
        arr_lib.register_function("join", crate::functions::arrlib::join);
        arr_lib.register_function("length", crate::functions::arrlib::length);
        // These functions are not implemented yet, so we'll comment them out for now
        // arr_lib.register_function("map", crate::functions::arrlib::map);
        // arr_lib.register_function("filter", crate::functions::arrlib::filter);
        arr_lib.register_function("unique", crate::functions::arrlib::unique);
        self.register_library(arr_lib);

        // String library
        let mut str_lib = Library::new("strlib");
        str_lib.register_function("upper", crate::functions::strlib::upper);
        str_lib.register_function("lower", crate::functions::strlib::lower);
        str_lib.register_function("substring", crate::functions::strlib::substring);
        str_lib.register_function("replace", crate::functions::strlib::replace);
        str_lib.register_function("length", crate::functions::strlib::length);
        str_lib.register_function("split", crate::functions::strlib::split);
        str_lib.register_function("trim", crate::functions::strlib::trim);
        str_lib.register_function("starts_with", crate::functions::strlib::starts_with);
        str_lib.register_function("ends_with", crate::functions::strlib::ends_with);
        str_lib.register_function("contains", crate::functions::strlib::contains);
        str_lib.register_function("repeat", crate::functions::strlib::repeat);
        self.register_library(str_lib);

        // Math library
        let mut math_lib = Library::new("mathlib");
        math_lib.register_function("add", crate::functions::mathlib::add);
        math_lib.register_function("subtract", crate::functions::mathlib::subtract);
        math_lib.register_function("multiply", crate::functions::mathlib::multiply);
        math_lib.register_function("divide", crate::functions::mathlib::divide);
        math_lib.register_function("power", crate::functions::mathlib::power);
        math_lib.register_function("sqrt", crate::functions::mathlib::sqrt);
        math_lib.register_function("abs", crate::functions::mathlib::abs);
        math_lib.register_function("round", crate::functions::mathlib::round);
        math_lib.register_function("floor", crate::functions::mathlib::floor);
        math_lib.register_function("ceil", crate::functions::mathlib::ceil);
        math_lib.register_function("sin", crate::functions::mathlib::sin);
        math_lib.register_function("cos", crate::functions::mathlib::cos);
        math_lib.register_function("tan", crate::functions::mathlib::tan);
        math_lib.register_function("log", crate::functions::mathlib::log);
        math_lib.register_function("exp", crate::functions::mathlib::exp);
        math_lib.register_function("random", crate::functions::mathlib::random);
        math_lib.register_function("max", crate::functions::mathlib::max);
        math_lib.register_function("min", crate::functions::mathlib::min);
        math_lib.register_function("modulo", crate::functions::mathlib::modulo);
        self.register_library(math_lib);

        // Time library
        let mut time_lib = Library::new("timelib");
        time_lib.register_function("now", crate::functions::timelib::now);
        time_lib.register_function("format", crate::functions::timelib::format);
        time_lib.register_function("parse", crate::functions::timelib::parse);
        time_lib.register_function("add", crate::functions::timelib::add);
        time_lib.register_function("year", crate::functions::timelib::year);
        time_lib.register_function("month", crate::functions::timelib::month);
        time_lib.register_function("day", crate::functions::timelib::day);
        self.register_library(time_lib);

        // Random library
        let mut random_lib = Library::new("random");
        random_lib.register_function("int", crate::functions::randomlib::int);
        random_lib.register_function("float", crate::functions::randomlib::float);
        random_lib.register_function("choice", crate::functions::randomlib::choice);
        random_lib.register_function("shuffle", crate::functions::randomlib::shuffle);
        self.register_library(random_lib);

        // File library
        let mut file_lib = Library::new("file");
        file_lib.register_function("read", crate::functions::filelib::read);
        file_lib.register_function("write", crate::functions::filelib::write);
        file_lib.register_function("append", crate::functions::filelib::append);
        file_lib.register_function("exists", crate::functions::filelib::exists);
        file_lib.register_function("delete", crate::functions::filelib::delete);
        self.register_library(file_lib);

        // JSON library
        let mut json_lib = Library::new("json");
        json_lib.register_function("parse", crate::functions::jsonlib::parse);
        json_lib.register_function("stringify", crate::functions::jsonlib::stringify);
        self.register_library(json_lib);

        // Network library
        let mut net_lib = Library::new("net");
        net_lib.register_function("ping", crate::functions::netlib::ping);
        self.register_library(net_lib);

        // Bolt library
        let mut bolt_lib = Library::new("bolt");
        bolt_lib.register_function("run", crate::functions::boltlib::run);
        bolt_lib.register_function("parallel", crate::functions::boltlib::parallel);
        self.register_library(bolt_lib);

        // Seed library
        let mut seed_lib = Library::new("seed");
        seed_lib.register_function("generate", crate::functions::seedlib::generate);
        seed_lib.register_function("map", crate::functions::seedlib::map);
        self.register_library(seed_lib);

        // Register color library functions
        let mut color_lib = Library::new("color");
        color_lib.register_function("hex_to_rgb", crate::functions::colorlib::hex_to_rgb);
        color_lib.register_function("rgb_to_hex", crate::functions::colorlib::rgb_to_hex);
        color_lib.register_function("lighten", crate::functions::colorlib::lighten);
        color_lib.register_function("darken", crate::functions::colorlib::darken);
        self.register_library(color_lib);

        // Register crypto library functions
        let mut crypto_lib = Library::new("crypto");
        crypto_lib.register_function("hash", crate::functions::cryptolib::hash);
        crypto_lib.register_function("encrypt", crate::functions::cryptolib::encrypt);
        crypto_lib.register_function("decrypt", crate::functions::cryptolib::decrypt);
        self.register_library(crypto_lib);

        // Register regex library functions
        let mut regex_lib = Library::new("regex");
        regex_lib.register_function("match", crate::functions::regexlib::match_pattern);
        regex_lib.register_function("search", crate::functions::regexlib::search);
        regex_lib.register_function("replace", crate::functions::regexlib::replace);
        self.register_library(regex_lib);

        // Register UUID library functions
        let mut uuid_lib = Library::new("uuid");
        uuid_lib.register_function("generate", crate::functions::uuidlib::generate);
        uuid_lib.register_function("parse", crate::functions::uuidlib::parse);
        uuid_lib.register_function("is_valid", crate::functions::uuidlib::is_valid);
        self.register_library(uuid_lib);

        // Register OS library functions
        let mut os_lib = Library::new("os");
        os_lib.register_function("env", crate::functions::oslib::env_var);
        os_lib.register_function("cwd", crate::functions::oslib::cwd);
        os_lib.register_function("platform", crate::functions::oslib::platform);
        self.register_library(os_lib);

        // Register Validation library functions
        let mut validation_lib = Library::new("validation");
        validation_lib.register_function("email", crate::functions::validationlib::email);
        validation_lib.register_function("phone", crate::functions::validationlib::phone);
        validation_lib.register_function("required", crate::functions::validationlib::required);
        validation_lib.register_function("min_length", crate::functions::validationlib::min_length);
        self.register_library(validation_lib);

        // Register System library functions
        let mut system_lib = Library::new("system");
        system_lib.register_function("exec", crate::functions::systemlib::exec);
        system_lib.register_function("uptime", crate::functions::systemlib::uptime);
        system_lib.register_function("info", crate::functions::systemlib::info);
        self.register_library(system_lib);
    }
}

// Global library manager instance
lazy_static::lazy_static! {
    static ref LIBRARY_MANAGER: std::sync::Mutex<LibraryManager> = std::sync::Mutex::new(LibraryManager::new());
}

/// Initialize the library system
pub fn initialize() {
    let mut manager = LIBRARY_MANAGER.lock().unwrap();
    manager.initialize_standard_libraries();
}

/// Call a library function
pub fn call_library(library_name: &str, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
    let manager = LIBRARY_MANAGER.lock().unwrap();
    manager.call_library(library_name, function_name, args)
}

/// Register a custom library
pub fn register_library(library: Library) {
    let mut manager = LIBRARY_MANAGER.lock().unwrap();
    manager.register_library(library);
}

/// Get a list of all registered libraries
pub fn get_library_names() -> Vec<String> {
    let manager = LIBRARY_MANAGER.lock().unwrap();
    manager.libraries.keys().cloned().collect()
}

/// Get a list of all functions in a library
pub fn get_library_functions(library_name: &str) -> Result<Vec<String>, String> {
    let manager = LIBRARY_MANAGER.lock().unwrap();
    match manager.get_library(library_name) {
        Some(library) => Ok(library.function_names()),
        None => Err(format!("Library '{}' not found", library_name)),
    }
}