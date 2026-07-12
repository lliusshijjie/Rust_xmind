use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use web_server::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878")
        .expect("failed to bind 127.0.0.1:7878");

    let pool = ThreadPool::new(4);

    println!("Server running at http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(move || {
                    handle_connection(stream);
                });
            }

            Err(error) => {
                eprintln!("Failed to accept connection: {error}");
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let request_line = {
        let reader = BufReader::new(&stream);

        match reader.lines().next() {
            Some(Ok(line)) => line,

            Some(Err(error)) => {
                eprintln!("Failed to read request: {error}");
                return;
            }

            None => {
                eprintln!("Received empty request");
                return;
            }
        }
    };

    println!("Request: {request_line}");

    let (status_line, filename) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),

        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }

        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = match fs::read_to_string(filename) {
        Ok(contents) => contents,

        Err(error) => {
            eprintln!("Failed to read {filename}: {error}");

            let body = "500 Internal Server Error";

            let response = format!(
                "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\
                 Content-Length: {}\r\n\
                 Content-Type: text/plain; charset=utf-8\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                body.len(),
                body
            );

            let _ = stream.write_all(response.as_bytes());
            return;
        }
    };

    let response = format!(
        "{status_line}\r\n\
         Content-Length: {}\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Connection: close\r\n\
         \r\n\
         {contents}",
        contents.len()
    );

    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("Failed to send response: {error}");
    }
}
