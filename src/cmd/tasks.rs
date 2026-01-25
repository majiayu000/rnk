//! Predefined task helpers for common side effects
//!
//! This module provides convenient helper functions for creating commands
//! for common tasks like HTTP requests, file I/O, and process spawning.

use super::Cmd;
use std::path::Path;
use std::process::Stdio;

/// HTTP request configuration
pub struct HttpRequest {
    /// The URL to request
    pub url: String,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request headers
    pub headers: Vec<(String, String)>,
    /// Request body (optional)
    pub body: Option<String>,
}

impl HttpRequest {
    /// Create a new GET request
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: "GET".to_string(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Create a new POST request
    pub fn post(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: "POST".to_string(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Add a header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set the request body
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

/// HTTP response
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: String,
}

impl Cmd {
    /// Create a command that performs an HTTP request
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rnk::cmd::{Cmd, HttpRequest};
    ///
    /// let cmd = Cmd::http(
    ///     HttpRequest::get("https://api.github.com/users/octocat"),
    ///     |response| {
    ///         println!("Status: {}", response.status);
    ///         println!("Body: {}", response.body);
    ///     },
    /// );
    /// ```
    pub fn http<F>(request: HttpRequest, on_response: F) -> Self
    where
        F: FnOnce(Result<HttpResponse, String>) + Send + 'static,
    {
        Cmd::perform(move || async move {
            // Build the request using reqwest (will be available in dependencies)
            #[cfg(feature = "http")]
            {
                use reqwest;

                let client = reqwest::Client::new();
                let mut req = match request.method.as_str() {
                    "GET" => client.get(&request.url),
                    "POST" => client.post(&request.url),
                    "PUT" => client.put(&request.url),
                    "DELETE" => client.delete(&request.url),
                    _ => client.get(&request.url),
                };

                // Add headers
                for (key, value) in request.headers {
                    req = req.header(key, value);
                }

                // Add body if present
                if let Some(body) = request.body {
                    req = req.body(body);
                }

                // Send request
                match req.send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        let headers = resp
                            .headers()
                            .iter()
                            .map(|(k, v)| {
                                (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())
                            })
                            .collect();

                        match resp.text().await {
                            Ok(body) => {
                                on_response(Ok(HttpResponse {
                                    status,
                                    headers,
                                    body,
                                }));
                            }
                            Err(e) => {
                                on_response(Err(format!("Failed to read response body: {}", e)));
                            }
                        }
                    }
                    Err(e) => {
                        on_response(Err(format!("HTTP request failed: {}", e)));
                    }
                }
            }

            #[cfg(not(feature = "http"))]
            {
                let _ = request;
                on_response(Err(
                    "HTTP support not enabled. Enable 'http' feature.".to_string()
                ));
            }
        })
    }

    /// Create a command that reads a file
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rnk::cmd::Cmd;
    ///
    /// let cmd = Cmd::read_file(
    ///     "config.json",
    ///     |result| {
    ///         match result {
    ///             Ok(contents) => println!("File contents: {}", contents),
    ///             Err(e) => eprintln!("Error reading file: {}", e),
    ///         }
    ///     },
    /// );
    /// ```
    pub fn read_file<P, F>(path: P, on_read: F) -> Self
    where
        P: AsRef<Path> + Send + 'static,
        F: FnOnce(Result<String, String>) + Send + 'static,
    {
        Cmd::perform(move || async move {
            match tokio::fs::read_to_string(path.as_ref()).await {
                Ok(contents) => on_read(Ok(contents)),
                Err(e) => on_read(Err(format!("Failed to read file: {}", e))),
            }
        })
    }

    /// Create a command that writes to a file
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rnk::cmd::Cmd;
    ///
    /// let cmd = Cmd::write_file(
    ///     "output.txt",
    ///     "Hello, world!",
    ///     |result| {
    ///         match result {
    ///             Ok(_) => println!("File written successfully"),
    ///             Err(e) => eprintln!("Error writing file: {}", e),
    ///         }
    ///     },
    /// );
    /// ```
    pub fn write_file<P, C, F>(path: P, contents: C, on_write: F) -> Self
    where
        P: AsRef<Path> + Send + 'static,
        C: AsRef<str> + Send + 'static,
        F: FnOnce(Result<(), String>) + Send + 'static,
    {
        Cmd::perform(move || async move {
            match tokio::fs::write(path.as_ref(), contents.as_ref()).await {
                Ok(_) => on_write(Ok(())),
                Err(e) => on_write(Err(format!("Failed to write file: {}", e))),
            }
        })
    }

    /// Create a command that spawns a process
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rnk::cmd::Cmd;
    ///
    /// let cmd = Cmd::spawn(
    ///     "ls",
    ///     vec!["-la"],
    ///     |result| {
    ///         match result {
    ///             Ok(output) => println!("Output: {}", output.stdout),
    ///             Err(e) => eprintln!("Error: {}", e),
    ///         }
    ///     },
    /// );
    /// ```
    pub fn spawn<S, F>(command: S, args: Vec<String>, on_complete: F) -> Self
    where
        S: AsRef<str> + Send + 'static,
        F: FnOnce(Result<ProcessOutput, String>) + Send + 'static,
    {
        Cmd::perform(move || async move {
            let mut cmd = tokio::process::Command::new(command.as_ref());
            cmd.args(&args);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            match cmd.output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = output.status.code().unwrap_or(-1);

                    on_complete(Ok(ProcessOutput {
                        stdout,
                        stderr,
                        exit_code,
                        success: output.status.success(),
                    }));
                }
                Err(e) => {
                    on_complete(Err(format!("Failed to spawn process: {}", e)));
                }
            }
        })
    }

    /// Create a command that runs a callback after a delay
    ///
    /// This is an alias for `Cmd::sleep().and_then(Cmd::perform(...))`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rnk::cmd::Cmd;
    /// use std::time::Duration;
    ///
    /// let cmd = Cmd::delay(Duration::from_secs(2), || async {
    ///     println!("Executed after 2 seconds");
    /// });
    /// ```
    pub fn delay<F, Fut>(duration: std::time::Duration, f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        Cmd::sleep(duration).and_then(Cmd::perform(f))
    }
}

/// Process output
pub struct ProcessOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
    /// Whether the process succeeded
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_http_request_builder() {
        let req = HttpRequest::get("https://example.com")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer token");

        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, "GET");
        assert_eq!(req.headers.len(), 2);
        assert_eq!(req.headers[0].0, "Content-Type");
        assert_eq!(req.headers[0].1, "application/json");
    }

    #[test]
    fn test_http_post_with_body() {
        let req = HttpRequest::post("https://api.example.com/data")
            .header("Content-Type", "application/json")
            .body(r#"{"key": "value"}"#);

        assert_eq!(req.method, "POST");
        assert!(req.body.is_some());
        assert_eq!(req.body.unwrap(), r#"{"key": "value"}"#);
    }

    #[tokio::test]
    async fn test_read_file() {
        use std::sync::{Arc, Mutex};

        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        // Create a temporary file
        let temp_file = std::env::temp_dir().join("rnk_test_read.txt");
        std::fs::write(&temp_file, "test content").unwrap();

        let cmd = Cmd::read_file(temp_file.clone(), move |res| {
            *result_clone.lock().unwrap() = Some(res);
        });

        // Execute the command
        if let Cmd::Perform { future } = cmd {
            future.await;
        }

        // Check result
        let res = result.lock().unwrap().take().unwrap();
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "test content");

        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[tokio::test]
    async fn test_write_file() {
        use std::sync::{Arc, Mutex};

        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        let temp_file = std::env::temp_dir().join("rnk_test_write.txt");

        let cmd = Cmd::write_file(temp_file.clone(), "hello world", move |res| {
            *result_clone.lock().unwrap() = Some(res);
        });

        // Execute the command
        if let Cmd::Perform { future } = cmd {
            future.await;
        }

        // Check result
        let res = result.lock().unwrap().take().unwrap();
        assert!(res.is_ok());

        // Verify file contents
        let contents = std::fs::read_to_string(&temp_file).unwrap();
        assert_eq!(contents, "hello world");

        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[tokio::test]
    async fn test_spawn_process() {
        use std::sync::{Arc, Mutex};

        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        let cmd = Cmd::spawn("echo", vec!["hello".to_string()], move |res| {
            *result_clone.lock().unwrap() = Some(res);
        });

        // Execute the command
        if let Cmd::Perform { future } = cmd {
            future.await;
        }

        // Check result
        let res = result.lock().unwrap().take().unwrap();
        assert!(res.is_ok());

        let output = res.unwrap();
        assert!(output.success);
        assert!(output.stdout.contains("hello"));
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn test_delay_command() {
        let cmd = Cmd::delay(Duration::from_millis(100), || async {
            println!("delayed");
        });

        // Should be a Sleep command
        assert!(matches!(cmd, Cmd::Sleep { .. }));
    }

    #[test]
    fn test_read_file_nonexistent() {
        use std::sync::{Arc, Mutex};

        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        let cmd = Cmd::read_file("/nonexistent/file.txt", move |res| {
            *result_clone.lock().unwrap() = Some(res);
        });

        // Execute using tokio runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Cmd::Perform { future } = cmd {
                future.await;
            }
        });

        // Should get an error
        let res = result.lock().unwrap().take().unwrap();
        assert!(res.is_err());
    }

    #[test]
    fn test_spawn_nonexistent_command() {
        use std::sync::{Arc, Mutex};

        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        let cmd = Cmd::spawn("nonexistent_command_xyz", vec![], move |res| {
            *result_clone.lock().unwrap() = Some(res);
        });

        // Execute using tokio runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Cmd::Perform { future } = cmd {
                future.await;
            }
        });

        // Should get an error
        let res = result.lock().unwrap().take().unwrap();
        assert!(res.is_err());
    }
}
