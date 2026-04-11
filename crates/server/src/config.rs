use std::env;

/// Server configuration parsed from CLI flags or environment variables.
#[derive(Debug)]
pub struct Config {
    pub port: u16,
    pub db_path: String,
    pub api_key: String,
}

impl Config {
    /// Build config from environment variables and/or command-line arguments.
    ///
    /// Priority for `api_key`: `--key` flag > `SYNC_API_KEY` env var.
    /// Priority for `port`: `--port` flag > `SYNC_PORT` env var > default 3000.
    /// Priority for `db_path`: `--db` flag > `SYNC_DB_PATH` env var > default "./sync.db".
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut port: Option<u16> = None;
        let mut db_path: Option<String> = None;
        let mut api_key: Option<String> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--port" => {
                    i += 1;
                    port = Some(
                        args.get(i)
                            .ok_or("--port requires a value")?
                            .parse::<u16>()
                            .map_err(|e| format!("Invalid port: {e}"))?,
                    );
                }
                "--db" => {
                    i += 1;
                    db_path = Some(
                        args.get(i)
                            .ok_or("--db requires a value")?
                            .to_string(),
                    );
                }
                "--key" => {
                    i += 1;
                    api_key = Some(
                        args.get(i)
                            .ok_or("--key requires a value")?
                            .to_string(),
                    );
                }
                other => {
                    return Err(format!("Unknown argument: {other}"));
                }
            }
            i += 1;
        }

        let api_key = api_key
            .or_else(|| env::var("SYNC_API_KEY").ok())
            .ok_or("API key is required. Use --key flag or SYNC_API_KEY env var")?;

        let port = port
            .or_else(|| env::var("SYNC_PORT").ok().and_then(|s| s.parse().ok()))
            .unwrap_or(3000);

        let db_path = db_path
            .or_else(|| env::var("SYNC_DB_PATH").ok())
            .unwrap_or_else(|| "./sync.db".to_string());

        Ok(Config {
            port,
            db_path,
            api_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_with_all_flags() {
        let args = vec![
            "--port".to_string(),
            "8080".to_string(),
            "--db".to_string(),
            "/tmp/test.db".to_string(),
            "--key".to_string(),
            "secret123".to_string(),
        ];
        let config = Config::parse(&args).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.db_path, "/tmp/test.db");
        assert_eq!(config.api_key, "secret123");
    }

    #[test]
    fn parse_defaults() {
        env::remove_var("SYNC_PORT");
        env::remove_var("SYNC_DB_PATH");
        env::set_var("SYNC_API_KEY", "envkey");

        let args = vec!["--key".to_string(), "testkey".to_string()];
        let config = Config::parse(&args).unwrap();
        assert_eq!(config.port, 3000);
        assert_eq!(config.db_path, "./sync.db");
        assert_eq!(config.api_key, "testkey");

        // Flag overrides env
        env::remove_var("SYNC_API_KEY");
    }

    #[test]
    fn parse_uses_env_api_key() {
        env::set_var("SYNC_API_KEY", "envkey");
        let config = Config::parse(&[]).unwrap();
        assert_eq!(config.api_key, "envkey");
        env::remove_var("SYNC_API_KEY");
    }

    #[test]
    fn parse_fails_without_api_key() {
        env::remove_var("SYNC_API_KEY");
        let result = Config::parse(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key is required"));
    }

    #[test]
    fn parse_uses_env_port() {
        env::set_var("SYNC_PORT", "9999");
        env::set_var("SYNC_API_KEY", "key");
        let config = Config::parse(&[]).unwrap();
        assert_eq!(config.port, 9999);
        env::remove_var("SYNC_PORT");
        env::remove_var("SYNC_API_KEY");
    }

    #[test]
    fn parse_flag_overrides_env() {
        env::set_var("SYNC_API_KEY", "envkey");
        let args = vec!["--key".to_string(), "flagkey".to_string()];
        let config = Config::parse(&args).unwrap();
        assert_eq!(config.api_key, "flagkey");
        env::remove_var("SYNC_API_KEY");
    }

    #[test]
    fn parse_unknown_arg() {
        env::set_var("SYNC_API_KEY", "key");
        let args = vec!["--unknown".to_string()];
        let result = Config::parse(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown argument"));
        env::remove_var("SYNC_API_KEY");
    }

    #[test]
    fn parse_invalid_port() {
        env::set_var("SYNC_API_KEY", "key");
        let args = vec!["--port".to_string(), "abc".to_string()];
        let result = Config::parse(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid port"));
        env::remove_var("SYNC_API_KEY");
    }
}
