---
title: Database
description: SQLite database operations in naml
---

## SQLite Demo

Full SQLite example covering table creation, queries, transactions, and prepared statements:

```naml
use std::db::sqlite::*;

fn main() {
    // Open in-memory database
    var db: int = open_memory() catch e {
        println("Error: {}", e.message);
        return;
    };

    // Create table
    exec(db, "CREATE TABLE users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        age INTEGER NOT NULL,
        score REAL
    )") catch e {
        println("Error: {}", e.message);
        return;
    };

    // Insert data
    exec(db, "INSERT INTO users (name, age, score) VALUES ('Alice', 30, 95.5)") catch e { return; };
    exec(db, "INSERT INTO users (name, age, score) VALUES ('Bob', 25, 87.3)") catch e { return; };
    exec(db, "INSERT INTO users (name, age, score) VALUES ('Charlie', 35, 92.1)") catch e { return; };

    println("Inserted {} row(s)", changes(db));
    println("Last insert ID: {}", last_insert_id(db));

    // Query all users
    var rows: int = query(db, "SELECT * FROM users", []) catch e {
        println("Error: {}", e.message);
        return;
    };

    var count: int = row_count(rows);
    println("Found {} users", count);

    var i: int = 0;
    while (i < count) {
        var row: int = row_at(rows, i);
        println("  {}: name={}, age={}, score={}",
            get_int(row, "id"),
            get_string(row, "name"),
            get_int(row, "age"),
            get_float(row, "score")
        );
        i = i + 1;
    }

    // Parameterized query
    var filtered: int = query(db, "SELECT name, age FROM users WHERE age > ?", ["28"]) catch e {
        println("Error: {}", e.message);
        return;
    };
    println("Users older than 28:");
    var j: int = 0;
    while (j < row_count(filtered)) {
        var frow: int = row_at(filtered, j);
        println("  {}: age {}", get_string(frow, "name"), get_int(frow, "age"));
        j = j + 1;
    }

    // Transaction
    begin(db) catch e { return; };
    exec(db, "INSERT INTO users (name, age, score) VALUES ('Dave', 28, 88.0)") catch e { return; };
    commit(db) catch e { return; };

    // Prepared statement
    var stmt: int = prepare(db, "INSERT INTO users (name, age, score) VALUES (?, ?, ?)") catch e {
        return;
    };

    bind_string(stmt, 1, "Eve") catch e { return; };
    bind_int(stmt, 2, 22) catch e { return; };
    bind_float(stmt, 3, 91.0) catch e { return; };
    step(stmt) catch e { return; };

    reset(stmt);

    bind_string(stmt, 1, "Frank") catch e { return; };
    bind_int(stmt, 2, 31) catch e { return; };
    bind_float(stmt, 3, 85.5) catch e { return; };
    step(stmt) catch e { return; };

    finalize(stmt);

    // Final count
    var final_rows: int = query(db, "SELECT * FROM users ORDER BY id", []) catch e { return; };
    println("Total users: {}", row_count(final_rows));

    // NULL check
    exec(db, "INSERT INTO users (name, age) VALUES ('NoScore', 40)") catch e { return; };
    var null_rows: int = query(db, "SELECT * FROM users WHERE name = ?", ["NoScore"]) catch e { return; };
    var null_row: int = row_at(null_rows, 0);
    println("NoScore's score is null: {}", is_null(null_row, "score"));

    close(db);
}
```

Key patterns:
- `open_memory()` for in-memory databases, `open(path)` for file-based
- `exec()` for statements without results (CREATE, INSERT, UPDATE, DELETE)
- `query()` for SELECT statements — returns a rows handle
- `row_count()` / `row_at()` to iterate results
- `get_string()` / `get_int()` / `get_float()` / `get_bool()` to extract column values
- `begin()` / `commit()` / `rollback()` for transactions
- `prepare()` / `bind_*()` / `step()` / `finalize()` for prepared statements
