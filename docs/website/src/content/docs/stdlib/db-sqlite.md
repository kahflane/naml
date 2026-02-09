---
title: "std::db::sqlite"
description: SQLite3 database integration
---

SQLite3 database integration for embedded SQL database support.

## Import

```naml
use std::db::sqlite::*;
```

## Error Handling

All database operations throw `DBError` on failure.

## Database Connection

### open

Open SQLite database file.

```naml
fn open(path: string) -> int throws DBError
```

**Returns:** Database handle.

**Example:**

```naml
var db: int = open("/tmp/mydb.db") catch e {
    println(e.message);
    return;
};
```

### open_memory

Open in-memory database.

```naml
fn open_memory() -> int throws DBError
```

**Example:**

```naml
var db: int = open_memory() catch e {
    println(e.message);
    return;
};
```

### close

Close database connection.

```naml
fn close(db: int)
```

**Example:**

```naml
close(db);
```

## Query Execution

### exec

Execute SQL statement without returning results.

```naml
fn exec(db: int, sql: string) throws DBError
```

**Example:**

```naml
exec(db, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)") catch e {
    println(e.message);
};
```

### query

Execute SQL query and return result set handle.

```naml
fn query(db: int, sql: string, params: [string]) -> int throws DBError
```

**Parameters:**
- `params` - Parameter values for `?` placeholders

**Returns:** Result set handle.

**Example:**

```naml
var rows: int = query(db, "SELECT * FROM users WHERE age > ?", ["25"]) catch e {
    println(e.message);
    return;
};
```

## Result Set Operations

### row_count

Get number of rows in result set.

```naml
fn row_count(rows: int) -> int
```

**Example:**

```naml
var count: int = row_count(rows);
```

### row_at

Get row handle at index.

```naml
fn row_at(rows: int, index: int) -> int
```

**Example:**

```naml
var row: int = row_at(rows, 0);
```

### get_string

Get string value from row by column name.

```naml
fn get_string(row: int, column: string) -> string
```

**Example:**

```naml
var name: string = get_string(row, "name");
```

### get_int

Get integer value from row by column name.

```naml
fn get_int(row: int, column: string) -> int
```

**Example:**

```naml
var age: int = get_int(row, "age");
```

### get_float

Get float value from row by column name.

```naml
fn get_float(row: int, column: string) -> float
```

**Example:**

```naml
var score: float = get_float(row, "score");
```

### get_bool

Get boolean value from row by column name.

```naml
fn get_bool(row: int, column: string) -> bool
```

**Example:**

```naml
var active: bool = get_bool(row, "active");
```

### is_null

Check if column value is NULL.

```naml
fn is_null(row: int, column: string) -> bool
```

**Example:**

```naml
if (is_null(row, "email")) {
    println("Email is NULL");
}
```

### columns

Get comma-separated column names.

```naml
fn columns(rows: int) -> string
```

**Example:**

```naml
var cols: string = columns(rows);  // "id,name,age"
```

### column_count

Get number of columns.

```naml
fn column_count(rows: int) -> int
```

**Example:**

```naml
var num_cols: int = column_count(rows);
```

## Transactions

### begin

Begin transaction.

```naml
fn begin(db: int) throws DBError
```

**Example:**

```naml
begin(db) catch e {
    println(e.message);
};
```

### commit

Commit transaction.

```naml
fn commit(db: int) throws DBError
```

**Example:**

```naml
commit(db) catch e {
    println(e.message);
};
```

### rollback

Rollback transaction.

```naml
fn rollback(db: int) throws DBError
```

**Example:**

```naml
rollback(db) catch e {
    println(e.message);
};
```

## Prepared Statements

### prepare

Prepare SQL statement.

```naml
fn prepare(db: int, sql: string) -> int throws DBError
```

**Returns:** Statement handle.

**Example:**

```naml
var stmt: int = prepare(db, "INSERT INTO users (name, age) VALUES (?, ?)") catch e {
    println(e.message);
    return;
};
```

### bind_string

Bind string parameter (1-indexed).

```naml
fn bind_string(stmt: int, index: int, value: string) throws DBError
```

**Example:**

```naml
bind_string(stmt, 1, "Alice") catch e {
    println(e.message);
};
```

### bind_int

Bind integer parameter (1-indexed).

```naml
fn bind_int(stmt: int, index: int, value: int) throws DBError
```

**Example:**

```naml
bind_int(stmt, 2, 30) catch e {
    println(e.message);
};
```

### bind_float

Bind float parameter (1-indexed).

```naml
fn bind_float(stmt: int, index: int, value: float) throws DBError
```

**Example:**

```naml
bind_float(stmt, 3, 95.5) catch e {
    println(e.message);
};
```

### step

Execute prepared statement.

```naml
fn step(stmt: int) throws DBError
```

**Example:**

```naml
step(stmt) catch e {
    println(e.message);
};
```

### reset

Reset prepared statement for reuse.

```naml
fn reset(stmt: int)
```

**Example:**

```naml
reset(stmt);
```

### finalize

Finalize and free prepared statement.

```naml
fn finalize(stmt: int)
```

**Example:**

```naml
finalize(stmt);
```

## Database Metadata

### changes

Get number of rows affected by last statement.

```naml
fn changes(db: int) -> int
```

**Example:**

```naml
exec(db, "DELETE FROM users WHERE age < 18") catch e {
    println(e.message);
};
var deleted: int = changes(db);
```

### last_insert_id

Get rowid of last inserted row.

```naml
fn last_insert_id(db: int) -> int
```

**Example:**

```naml
exec(db, "INSERT INTO users (name) VALUES ('Bob')") catch e {
    println(e.message);
};
var id: int = last_insert_id(db);
```

## Complete Example

```naml
use std::db::sqlite::*;

fn main() {
    var db: int = open_memory() catch e {
        println(fmt("Error: {}", e.message));
        return;
    };

    exec(db, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)") catch e {
        println(e.message);
        return;
    };

    exec(db, "INSERT INTO users (name, age) VALUES ('Alice', 30)") catch e {
        println(e.message);
        return;
    };

    exec(db, "INSERT INTO users (name, age) VALUES ('Bob', 25)") catch e {
        println(e.message);
        return;
    };

    var rows: int = query(db, "SELECT * FROM users", []) catch e {
        println(e.message);
        return;
    };

    var count: int = row_count(rows);
    var i: int = 0;
    while (i < count) {
        var row: int = row_at(rows, i);
        var id: int = get_int(row, "id");
        var name: string = get_string(row, "name");
        var age: int = get_int(row, "age");
        println(fmt("User {}: name={}, age={}", id, name, age));
        i = i + 1;
    }

    close(db);
}
```
