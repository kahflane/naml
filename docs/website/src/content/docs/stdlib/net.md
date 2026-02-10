---
title: "std::net"
description: TCP, UDP, HTTP, and TLS networking APIs
---

Networking APIs for TCP, UDP, HTTP, and TLS communication.

## Import

```naml
use std::net::tcp::*;           // TCP client/server
use std::net::udp::*;           // UDP sockets
use std::net::http::client::*;  // HTTP client
use std::net::http::server::*;  // HTTP server
use std::net::tls::*;           // TLS client/server
```

## TCP Server

### listen

Create TCP listener on address:port.

```naml
fn listen(addr: string, port: int) -> int throws NetworkError
```

**Example:**

```naml
var listener: int = listen("0.0.0.0", 8080) catch e {
    println(e.message);
    return;
};
```

### accept

Accept incoming connection, returns client socket.

```naml
fn accept(listener: int) -> int throws NetworkError
```

**Example:**

```naml
var client: int = accept(listener) catch e {
    println(e.message);
    return;
};
```

### close

Close TCP listener or connection.

```naml
fn close(socket: int) throws NetworkError
```

**Example:**

```naml
close(listener) catch e {
    println(e.message);
};
```

### local_addr

Get local address as string.

```naml
fn local_addr(socket: int) -> string throws NetworkError
```

**Example:**

```naml
var addr: string = local_addr(listener) catch e {
    println(e.message);
    return;
};
```

## TCP Client

### connect

Connect to remote TCP server.

```naml
fn connect(host: string, port: int) -> int throws NetworkError
```

**Example:**

```naml
var conn: int = connect("example.com", 80) catch e {
    println(e.message);
    return;
};
```

### read

Read up to n bytes from connection.

```naml
fn read(conn: int, size: int) -> bytes throws NetworkError
```

**Example:**

```naml
var data: bytes = read(conn, 1024) catch e {
    println(e.message);
    return;
};
```

### read_all

Read until EOF.

```naml
fn read_all(conn: int) -> bytes throws NetworkError
```

**Example:**

```naml
var data: bytes = read_all(conn) catch e {
    println(e.message);
    return;
};
```

### write

Write bytes to connection.

```naml
fn write(conn: int, data: bytes) -> int throws NetworkError
```

**Returns:** Number of bytes written.

**Example:**

```naml
var request: bytes = "GET / HTTP/1.0\r\n\r\n" as bytes;
var written: int = write(conn, request) catch e {
    println(e.message);
    return;
};
```

### set_timeout

Set read/write timeout in milliseconds.

```naml
fn set_timeout(conn: int, ms: int) throws NetworkError
```

**Example:**

```naml
set_timeout(conn, 5000) catch e {
    println(e.message);
};
```

### peer_addr

Get remote peer address.

```naml
fn peer_addr(conn: int) -> string throws NetworkError
```

**Example:**

```naml
var peer: string = peer_addr(conn) catch e {
    println(e.message);
    return;
};
```

## UDP

### bind

Bind UDP socket to address:port.

```naml
fn bind(addr: string, port: int) -> int throws NetworkError
```

**Example:**

```naml
var socket: int = bind("0.0.0.0", 9000) catch e {
    println(e.message);
    return;
};
```

### send

Send UDP packet to address.

```naml
fn send(socket: int, data: bytes, addr: string, port: int) -> int throws NetworkError
```

**Returns:** Number of bytes sent.

**Example:**

```naml
var sent: int = send(socket, "hello" as bytes, "127.0.0.1", 9001) catch e {
    println(e.message);
    return;
};
```

### receive

Receive UDP packet.

```naml
fn receive(socket: int, size: int) -> (bytes, string, int) throws NetworkError
```

**Returns:** Tuple of (data, sender_addr, sender_port).

**Example:**

```naml
var result: (bytes, string, int) = receive(socket, 1024) catch e {
    println(e.message);
    return;
};
var data: bytes = result.0;
var from_addr: string = result.1;
var from_port: int = result.2;
```

## HTTP Client

All HTTP client functions throw `NetworkError` on failure.

### get

Send HTTP GET request.

```naml
fn get(url: string, headers: option<map<string, string>>) -> int throws NetworkError
```

**Returns:** Response handle.

**Example:**

```naml
var response: int = get("http://example.com", none) catch e {
    println(e.message);
    return;
};
```

### post

Send HTTP POST request.

```naml
fn post(url: string, body: bytes, headers: option<map<string, string>>) -> int throws NetworkError
```

**Example:**

```naml
var json_body: bytes = `{"name":"Alice"}` as bytes;
var response: int = post("http://api.example.com/users", json_body, none) catch e {
    println(e.message);
    return;
};
```

### put

Send HTTP PUT request.

```naml
fn put(url: string, body: bytes, headers: option<map<string, string>>) -> int throws NetworkError
```

**Example:**

```naml
var response: int = put("http://api.example.com/users/1", body, none) catch e {
    println(e.message);
    return;
};
```

### patch

Send HTTP PATCH request.

```naml
fn patch(url: string, body: bytes, headers: option<map<string, string>>) -> int throws NetworkError
```

**Example:**

```naml
var response: int = patch("http://api.example.com/users/1", body, none) catch e {
    println(e.message);
    return;
};
```

### delete

Send HTTP DELETE request.

```naml
fn delete(url: string, headers: option<map<string, string>>) -> int throws NetworkError
```

**Example:**

```naml
var response: int = delete("http://api.example.com/users/1", none) catch e {
    println(e.message);
    return;
};
```

### set_timeout (HTTP)

Set HTTP client timeout in milliseconds.

```naml
fn set_timeout(ms: int)
```

**Example:**

```naml
set_timeout(5000);  // 5 second timeout
```

### status

Get HTTP response status code.

```naml
fn status(response: int) -> int
```

**Example:**

```naml
var code: int = status(response);  // 200, 404, etc.
```

### body

Get HTTP response body.

```naml
fn body(response: int) -> bytes
```

**Example:**

```naml
var response_body: bytes = body(response);
var text: string = response_body as string;
```

### HTTP Client Example

```naml
use std::net::http::client::{get, status, body};

fn main() {
    var response: int = get("http://httpbin.org/get", none) catch e {
        println(fmt("Error: {}", e.message));
        return;
    };

    var code: int = status(response);
    var data: bytes = body(response);

    println(fmt("Status: {}", code));
    println(fmt("Body: {}", data as string));
}
```

## HTTP Server

### open_router

Create HTTP router.

```naml
fn open_router() -> int
```

**Returns:** Router handle.

**Example:**

```naml
var router: int = open_router();
```

### get (route)

Register GET route handler.

```naml
fn get(router: int, path: string, handler: fn(int) -> int)
```

**Example:**

```naml
get(router, "/hello", fn(req: int) -> int {
    return text_response(200, "Hello, World!");
});
```

### post (route)

Register POST route handler.

```naml
fn post(router: int, path: string, handler: fn(int) -> int)
```

**Example:**

```naml
post(router, "/users", fn(req: int) -> int {
    return text_response(201, "User created");
});
```

### put (route)

Register PUT route handler.

```naml
fn put(router: int, path: string, handler: fn(int) -> int)
```

### patch (route)

Register PATCH route handler.

```naml
fn patch(router: int, path: string, handler: fn(int) -> int)
```

### delete (route)

Register DELETE route handler.

```naml
fn delete(router: int, path: string, handler: fn(int) -> int)
```

### with

Register middleware.

```naml
fn with(router: int, middleware: fn(int, fn(int) -> int) -> int)
```

**Example:**

```naml
with(router, fn(req: int, next: fn(int) -> int) -> int {
    println("Request received");
    return next(req);
});
```

### group

Create route group with prefix.

```naml
fn group(router: int, prefix: string) -> int
```

**Returns:** Group handle.

**Example:**

```naml
var api: int = group(router, "/api");
get(api, "/users", handler);  // Matches /api/users
```

### mount

Mount sub-router at path.

```naml
fn mount(router: int, path: string, subrouter: int)
```

**Example:**

```naml
var api_router: int = open_router();
mount(router, "/api", api_router);
```

### serve

Start HTTP server.

```naml
fn serve(router: int, addr: string, port: int) throws NetworkError
```

**Example:**

```naml
serve(router, "0.0.0.0", 8080) catch e {
    println(fmt("Server error: {}", e.message));
};
```

### text_response

Create text response.

```naml
fn text_response(status: int, body: string) -> int
```

**Example:**

```naml
return text_response(200, "OK");
```

### HTTP Server Example

```naml
use std::net::http::server::*;

fn main() {
    var router: int = open_router();

    get(router, "/", fn(req: int) -> int {
        return text_response(200, "Welcome!");
    });

    get(router, "/hello/:name", fn(req: int) -> int {
        return text_response(200, "Hello!");
    });

    serve(router, "0.0.0.0", 8080) catch e {
        println(fmt("Error: {}", e.message));
    };
}
```

## TLS Client

TLS connections using rustls with Mozilla root CAs. HTTPS is also supported transparently via the HTTP client above.

### connect

Connect to a TLS server. Uses system root CAs for certificate verification.

```naml
fn connect(address: string) -> int throws TlsError, NetworkError
```

**Example:**

```naml
var socket: int = connect("example.com:443") catch e {
    println(fmt("TLS connect failed: {}", e.message));
    return;
};
```

### read

Read up to n bytes from TLS connection.

```naml
fn read(socket: int, size: int) -> bytes throws NetworkError
```

### read_all

Read all data until EOF.

```naml
fn read_all(socket: int) -> bytes throws NetworkError
```

### write

Write bytes over TLS connection.

```naml
fn write(socket: int, data: bytes) throws NetworkError
```

### close

Close TLS connection.

```naml
fn close(socket: int)
```

### set_timeout

Set read/write timeout in milliseconds.

```naml
fn set_timeout(socket: int, ms: int)
```

### peer_addr

Get remote peer address.

```naml
fn peer_addr(socket: int) -> string
```

### TLS Client Example

```naml
use std::net::tls::{connect, read, write, close, set_timeout, peer_addr};

fn main() {
    var socket: int = connect("example.com:443") catch e {
        println(fmt("TLS connect failed: {}", e.message));
        return;
    };

    println(fmt("Connected to: {}", peer_addr(socket)));
    set_timeout(socket, 10000);

    var request: bytes = "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n" as bytes;
    write(socket, request) catch e {
        println(fmt("Write failed: {}", e.message));
        close(socket);
        return;
    };

    var response: bytes = read(socket, 4096) catch e {
        println(fmt("Read failed: {}", e.message));
        close(socket);
        return;
    };

    println(response as string);
    close(socket);
}
```

## TLS Server

Wrap a TCP listener with TLS using PEM certificate and key files.

### wrap_listener

Create a TLS listener from an existing TCP listener.

```naml
fn wrap_listener(listener: int, cert_path: string, key_path: string) -> int throws TlsError
```

| Param | Type | Description |
|-------|------|-------------|
| listener | int | TCP listener handle from `tcp::listen()` |
| cert_path | string | Path to PEM certificate file |
| key_path | string | Path to PEM private key file |

**Returns:** TLS listener handle.

### accept

Accept incoming TLS connection.

```naml
fn accept(tls_listener: int) -> int throws NetworkError, TlsError
```

**Returns:** TLS connection handle (use with TLS `read`/`write`/`close`).

### close_listener

Close TLS listener.

```naml
fn close_listener(tls_listener: int)
```

### TLS Server Example

```naml
use std::net::tcp::listen;
use std::net::tls::{wrap_listener, accept, read, write, close, close_listener};

fn main() {
    var tcp: int = listen("0.0.0.0", 8443) catch e {
        println(fmt("Listen failed: {}", e.message));
        return;
    };

    var tls: int = wrap_listener(tcp, "cert.pem", "key.pem") catch e {
        println(fmt("TLS setup failed: {}", e.message));
        return;
    };

    println("TLS server listening on :8443");

    while (true) {
        var client: int = accept(tls) catch e {
            println(fmt("Accept failed: {}", e.message));
            continue;
        };

        var data: bytes = read(client, 4096) catch e {
            close(client);
            continue;
        };

        write(client, "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK" as bytes) catch e {
            close(client);
            continue;
        };

        close(client);
    }

    close_listener(tls);
}
```

## HTTPS

The HTTP client supports HTTPS URLs transparently. No additional imports needed.

```naml
use std::net::http::client::{get, status, body};

fn main() {
    var response: int = get("https://example.com", none) catch e {
        println(fmt("HTTPS request failed: {}", e.message));
        return;
    };

    println(fmt("Status: {}", status(response)));
    println(body(response) as string);
}
```
