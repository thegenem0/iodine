use iodine_common::error::Error;

use crate::schema::RegistryConfig;

pub fn parse_yaml(yaml_str: &str) -> Result<RegistryConfig, Error> {
    let registry: RegistryConfig = serde_yaml::from_str(yaml_str).map_err(|e| {
        let err = if let Some(line) = e.location() {
            ParseError::InvalidYaml {
                line: line.line(),
                column: line.column(),
                message: e.to_string(),
            }
        } else {
            ParseError::InvalidYamlNoLocation {
                message: e.to_string(),
            }
        };
        Error::Internal(err.to_string())
    })?;

    Ok(registry)
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid YAML config at line {line}, column {column}: {message}")]
    InvalidYaml {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Invalid YAML config: {message}")]
    InvalidYamlNoLocation { message: String },
}
