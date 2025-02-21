use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

#[derive(Debug, Deserialize)]
struct Request {
    function: String,
    args: Args,
}

#[derive(Debug, Deserialize)]
struct Args {
    a: f64,
    b: f64,
}

#[derive(Debug, Serialize)]
struct Response {
    result: f64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = stdin.lock();

    loop {
        let mut input = String::new();
        if reader.read_line(&mut input)? == 0 {
            // EOF reached
            break;
        }

        // Parse the input JSON
        let request: Request = match serde_json::from_str(&input) {
            Ok(req) => req,
            Err(e) => {
                let error = ErrorResponse {
                    error: format!("Invalid request: {}", e),
                };
                serde_json::to_writer(&mut stdout, &error)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
                continue;
            }
        };

        // Process the request
        let result = match request.function.as_str() {
            "add" => request.args.a + request.args.b,
            "subtract" => request.args.a - request.args.b,
            "multiply" => request.args.a * request.args.b,
            "divide" => {
                if request.args.b == 0.0 {
                    let error = ErrorResponse {
                        error: "Division by zero".to_string(),
                    };
                    serde_json::to_writer(&mut stdout, &error)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                    continue;
                }
                request.args.a / request.args.b
            }
            _ => {
                let error = ErrorResponse {
                    error: format!("Unknown function: {}", request.function),
                };
                serde_json::to_writer(&mut stdout, &error)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
                continue;
            }
        };

        // Send the response
        let response = Response { result };
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let input = r#"{"function": "add", "args": {"a": 5.0, "b": 3.0}}"#;
        let request: Request = serde_json::from_str(input).unwrap();
        assert_eq!(request.args.a + request.args.b, 8.0);
    }

    #[test]
    fn test_divide_by_zero() {
        let input = r#"{"function": "divide", "args": {"a": 1.0, "b": 0.0}}"#;
        let request: Request = serde_json::from_str(input).unwrap();
        assert_eq!(request.args.b, 0.0);
    }
}
