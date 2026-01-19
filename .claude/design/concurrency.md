# Concurrency Model (Go-like)

## Overview

naml provides Go-like concurrency primitives that compile to Rust's tokio async runtime.

## Spawn (Goroutines)

### naml
```naml
spawn {
    println("Running in background");
    expensive_computation();
}
```

### Generated Rust
```rust
tokio::spawn(async {
    println!("Running in background");
    expensive_computation();
});
```

## Channels

### naml
```naml
// Create buffered channel
var ch = channel<int>(10);

// Sender
spawn {
    for (i in 0..100) {
        ch.send(i);
    }
    ch.close();
}

// Receiver
for (value in ch) {
    println(value);
}
```

### Generated Rust
```rust
let (tx, mut rx) = tokio::sync::mpsc::channel::<i64>(10);

tokio::spawn(async move {
    for i in 0..100 {
        tx.send(i).await.unwrap();
    }
    drop(tx); // close channel
});

while let Some(value) = rx.recv().await {
    println!("{}", value);
}
```

## Async/Await

### naml
```naml
async fn fetch_data(url: string) -> string {
    var response = await http_get(url);
    return response.body;
}

fn main() {
    var data = await fetch_data("https://api.example.com");
    println(data);
}
```

### Generated Rust
```rust
async fn fetch_data(url: String) -> String {
    let response = http_get(url).await;
    response.body
}

#[tokio::main]
async fn main() {
    let data = fetch_data("https://api.example.com".to_string()).await;
    println!("{}", data);
}
```

## Select (Channel Multiplexing)

### naml (future)
```naml
select {
    case value = ch1.receive(): {
        println("From ch1:", value);
    }
    case value = ch2.receive(): {
        println("From ch2:", value);
    }
    case timeout(1000): {
        println("Timeout!");
    }
}
```

### Generated Rust
```rust
tokio::select! {
    value = rx1.recv() => {
        if let Some(v) = value {
            println!("From ch1: {}", v);
        }
    }
    value = rx2.recv() => {
        if let Some(v) = value {
            println!("From ch2: {}", v);
        }
    }
    _ = tokio::time::sleep(Duration::from_millis(1000)) => {
        println!("Timeout!");
    }
}
```

## Mutex

### naml
```naml
var counter = mutex<int>(0);

spawn {
    var value = counter.lock();
    value = value + 1;
    counter.unlock();
}
```

### Generated Rust
```rust
let counter = Arc::new(Mutex::new(0i64));

let counter_clone = counter.clone();
tokio::spawn(async move {
    let mut value = counter_clone.lock().await;
    *value += 1;
});
```

## Runtime Requirements

Generated Rust code requires:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

For async main:
```rust
#[tokio::main]
async fn main() {
    // ...
}
```
