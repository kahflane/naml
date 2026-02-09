---
title: Exceptions
description: Learn about exception definitions, throwing, catching, and error handling in naml
---

naml provides structured exception handling for managing errors. Exceptions are explicit in function signatures and must be handled by callers.

## Exception Definition

Define exceptions using the `exception` keyword with optional fields:

```naml
exception DivisionByZero {
    dividend: int
}

exception ValidationError {
    field: string,
    message: string
}

exception NetworkError {
    code: int,
    reason: string
}
```

### Exceptions without Fields

Exceptions can be defined without fields:

```naml
exception NotFound {
}

exception Unauthorized {
}
```

## Throwing Exceptions

### Basic Throw

Use the `throw` keyword to raise an exception:

```naml
fn divide(a: int, b: int) -> int throws DivisionByZero {
    if (b == 0) {
        throw DivisionByZero("Cannot divide by zero");
    }
    return a / b;
}
```

### Throwing with Fields

Populate exception fields before throwing:

```naml
fn divide(a: int, b: int) -> int throws DivisionByZero {
    if (b == 0) {
        var ex: DivisionByZero = DivisionByZero("Cannot divide by zero");
        ex.dividend = a;
        throw ex;
    }
    return a / b;
}
```

### Multiple Exception Types

Functions can throw multiple exception types:

```naml
fn process(input: string) -> int throws ParseError, ValidationError {
    if (input == "") {
        var ex: ValidationError = ValidationError("Input cannot be empty");
        ex.field = "input";
        ex.message = "Empty string not allowed";
        throw ex;
    }

    var result: option<int> = input as? int;
    if (result == none) {
        throw ParseError("Invalid integer format");
    }

    return result!;
}
```

## Catching Exceptions

### Catch Block

Use `catch` to handle exceptions:

```naml
var result: int = divide(10, 0) catch e {
    println(e.message);
    return; // stop executing
} ?? -1;
```

The `catch` block receives the exception in variable `e`, which has a `message` field containing the error message.

### Accessing Exception Fields

Access custom exception fields in the catch block:

```naml
var result: int = divide(10, 0) catch e {
    println(fmt("Error: {}", e.message));
    println(fmt("Dividend was: {}", e.dividend));
    return;
} ?? 0;
```

### Multiple Handlers

When a function throws multiple exception types, catch handles all of them:

```naml
var value: int = process(input) catch e {
    // e is the caught exception (ParseError or ValidationError)
    println(fmt("Processing failed: {}", e.message));
    return;
} ?? 0;
```

## Try Expression

The `try` keyword can be used to propagate exceptions:

```naml
fn risky_operation() -> int throws SomeError {
    // ...
}

fn caller() -> int throws SomeError {
    var value: int = try risky_operation();
    return value;
}
```

## Exception Handling Patterns

### Early Return on Error

```naml
fn process_file(path: string) -> string throws FileError, ParseError {
    var content: string = read_file(path) catch e {
        println(fmt("Failed to read file: {}", e.message));
        return;
    } ?? "";

    if (content == "") {
        throw FileError("Empty file");
    }

    return content;
}
```

### Default Values

Provide default values when exceptions occur:

```naml
fn get_count(data: string) -> int {
    var count: int = parse_count(data) catch e {
        warn(fmt("Parse error: {}", e.message));
        return;
    } ?? 0;

    return count;
}
```

### Logging and Rethrowing

```naml
fn wrapper() -> int throws CustomError {
    var result: int = risky_call() catch e {
        error(fmt("Operation failed: {}", e.message));
        throw CustomError(e.message);
    } ?? 0;

    return result;
}
```

## Complete Example

```naml
// Exception definitions
exception InvalidInput {
    field: string,
    value: string
}

exception OutOfRange {
    min: int,
    max: int,
    actual: int
}

// Function that throws exceptions
fn validate_age(age_str: string) -> int throws InvalidInput, OutOfRange {
    // Parse the string
    var age: option<int> = age_str as? int;
    if (age == none) {
        var ex: InvalidInput = InvalidInput("Invalid age format");
        ex.field = "age";
        ex.value = age_str;
        throw ex;
    }

    var age_val: int = age!;

    // Validate range
    if (age_val < 0 or age_val > 150) {
        var ex: OutOfRange = OutOfRange("Age out of valid range");
        ex.min = 0;
        ex.max = 150;
        ex.actual = age_val;
        throw ex;
    }

    return age_val;
}

// Function that handles exceptions
fn process_age(input: string) -> string {
    var age: int = validate_age(input) catch e {
        // Handle both InvalidInput and OutOfRange
        println(fmt("Error: {}", e.message));
        return;
    } ?? -1;

    if (age == -1) {
        return "Invalid age";
    }

    if (age >= 18) {
        return "Adult";
    } else {
        return "Minor";
    }
}

fn main() {
    println(process_age("25"));      // "Adult"
    println(process_age("15"));      // "Minor"
    println(process_age("invalid")); // Error logged, "Invalid age"
    println(process_age("200"));     // Error logged, "Invalid age"
}
```

## Exception Propagation Example

```naml
exception NetworkError {
    code: int
}

exception DatabaseError {
    table: string
}

fn fetch_data() -> string throws NetworkError {
    // Simulate network call
    throw NetworkError("Connection failed");
}

fn save_data(data: string) throws DatabaseError {
    // Simulate database save
    throw DatabaseError("Table locked");
}

fn process() -> bool throws NetworkError, DatabaseError {
    var data: string = try fetch_data();  // Propagates NetworkError
    try save_data(data);                   // Propagates DatabaseError
    return true;
}

fn main() {
    var success: bool = process() catch e {
        error(fmt("Operation failed: {}", e.message));
        return;
    } ?? false;

    if (success) {
        println("Success");
    } else {
        println("Failed");
    }
}
```

## Best Practices

1. **Be specific** - Define specific exception types for different error conditions
2. **Use meaningful names** - Exception names should describe the error clearly
3. **Include context** - Add fields to exceptions to provide debugging information
4. **Document throws** - Always declare `throws` in function signatures
5. **Handle or propagate** - Either catch exceptions or declare them in `throws`
6. **Avoid silent failures** - Log errors even when providing default values
