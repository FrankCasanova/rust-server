use rust_server::ThreadPool;
use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn main() {
    // Create a TcpListener instance, which listens for incoming
    // connections. The argument is the address to listen on.
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    // Create a ThreadPool instance with four threads. It's a
    // good idea to create this early in `main()` so that any
    // configuration (like logging) is set up before the pool
    // is used.
    let pool = ThreadPool::new(4);

    // Get an iterator over incoming connections
    for stream in listener.incoming() {
        // `stream` is a `Result<TcpStream>` because it may not
        // be possible to create a `TcpStream` from the incoming
        // connection.
        let stream = stream.unwrap();

        // Submit a job to the thread pool. The closure passed to
        // `execute()` is the code that will be run by one of the
        // threads in the pool. The `move` keyword ensures that the
        // closure takes ownership of `stream` and thus that the
        // `handle_connection()` function will get a `TcpStream`
        // argument.
        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    // Take the stream and wrap it in a BufReader.
    //
    // `BufReader` is a type from the standard library that wraps a
    // `Read` object and provides buffering. It's useful for the
    // `lines()` method, which returns an iterator over the lines
    // of text in the stream.
    let buf_reader = BufReader::new(&stream);

    // Get the first line from the stream. This is the request line.
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    // Check the request line and return the appropriate response.
    //
    // The first arm of the `match` checks if the request line is
    // "GET / HTTP/1.1" and if so, returns a tuple containing the
    // status line and the filename that should be read.
    //
    // The second arm of the `match` checks if the request line is
    // "GET /sleep HTTP/1.1" and if so, it sleeps for five seconds
    // and then returns the same tuple as the first arm.
    //
    // The third arm of the `match` is the default arm and is
    // executed if the request line is neither of the above. It
    // returns a tuple containing a 404 status line and the
    // filename "404.html".
    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "index.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "index.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    // Read the contents of the file with the given filename.
    let contents = fs::read_to_string(filename).unwrap();

    // Get the length of the contents.
    let length = contents.len();

    // Create the response string.
    //
    // The first line of the response is the status line, which
    // contains the HTTP protocol, the status code, and a
    // description of the status code.
    //
    // The second line of the response contains the length of the
    // response body, which is the contents of the file.
    //
    // The third line of the response is a blank line, which
    // indicates that the response headers are finished.
    //
    // The fourth line of the response is the response body, which
    // is the contents of the file.
    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
    );

    // Write the response to the stream.
    stream.write_all(response.as_bytes()).unwrap();
}
