---
title: Networking
description: HTTP client, TCP, and UDP networking in naml
---

## HTTP Client

Making HTTP requests with all methods, concurrent requests, and shared statistics:

```naml
use std::threads::{join, with_mutex};
use std::net::http::client::{get, post, put, delete, status, body};
use std::strings::len;

fn main() {
    var success_count: mutex<int> = with_mutex(0);

    // GET request
    println("GET request:");
    var response: int = get("http://httpbin.org/get", none) catch e {
        println("Error: {}", e.message);
        return;
    };
    var code: int = status(response);
    var data: bytes = body(response);
    println("  Status: {}", code);
    println("  Body length: {} bytes", len(data as string));

    if (code >= 200 && code < 300) {
        locked (s: int in success_count) { s = s + 1; }
    }

    // POST request with JSON body
    println("POST request:");
    var post_body: bytes = `{"name":"Alice","age":30}` as bytes;
    var post_resp: int = post("http://httpbin.org/post", post_body, none) catch e {
        println("Error: {}", e.message);
        return;
    };
    println("  Status: {}", status(post_resp));

    // Concurrent requests
    println("3 parallel GETs:");
    spawn {
        get("http://httpbin.org/get?id=1", none) catch e { return; };
        locked (s: int in success_count) { s = s + 1; }
    };
    spawn {
        get("http://httpbin.org/get?id=2", none) catch e { return; };
        locked (s: int in success_count) { s = s + 1; }
    };
    spawn {
        get("http://httpbin.org/get?id=3", none) catch e { return; };
        locked (s: int in success_count) { s = s + 1; }
    };
    join();

    locked (s: int in success_count) {
        println("Successful requests: {}", s);
    }
}
```

Available HTTP methods: `get`, `post`, `put`, `patch`, `delete`. All return a response handle. Use `status(response)` for the status code and `body(response)` for the response body as `bytes`.
