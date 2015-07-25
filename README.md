# Tengas

An attempt at a [Hyper](http://hyper.rs) compatible async http client.

this library uses Hyper to handle parsing and serializing of http messages and 
it uses [mio](https://github.com/carllerche/mio) to manage the event loop.

## Currently, only GET requests are supported by the client

## Example

```rust
let mut client = Client::new().ok().expect("unable to start client");

let print_body = move|response: String| { 
  println!("{}", response)
};

client.get("http://httpbin.org/get", Box::new(print_body));

// because we are asynchronous, we need to sleep a bit to wait for the 
// callback to be reached!
thread::sleep_ms(1000);
```
