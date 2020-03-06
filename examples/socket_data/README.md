# Shared State

A simple introduction to how to pass state from a socket acceptor to the Gotham
state.

## Running

From the `examples/pre_state_data` directory:

```
Terminal 1:
$ cargo run
   Compiling gotham_examples_pre_state_data v0.0.0 (/Users/torozco/dev/gotham/examples/pre_state_data)
    Finished dev [unoptimized + debuginfo] target(s) in 0.24s
     Running `/Users/torozco/dev/gotham/target/debug/gotham_examples_pre_state_data`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.0
> Accept: */*
>
< HTTP/1.1 200 OK
< x-request-id: faba14b3-d777-4861-b0c3-63b77f0e3539
< content-type: text/plain
< content-length: 40
< date: Tue, 05 Nov 2019 14:46:00 GMT
<
You are connected to V4(127.0.0.1:7878)
* Connection #0 to host 127.0.0.1 left intact
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
