use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug, Clone)]
pub enum SageError {
    // Git-related errors
    GitNoStagedChanges,
    GitNoChanges,
    GitStagingFailed(String),
    GitCommitFailed(String),
    GitPushFailed(String),
    GitDiffFailed(String),

    // Configuration errors
    ConfigInvalidJson(String),
    ConfigApiKeyNotSet { provider: String },
    ConfigProviderNotFound { provider: String },
    ConfigProviderNotConfigured { provider: String },
    ConfigHomeDirNotFound,

    // API errors
    ApiNetworkError { provider: String, details: String },
    ApiAuthError { provider: String },
    ApiResponseError { provider: String, details: String },
    ApiNoResponse { provider: String },
    ApiUnsupportedProvider { provider: String },

    // I/O errors
    IoError(String),

    // Editor errors
    EditorFailed,

    // User input errors
    InvalidInput(String),
}

impl fmt::Display for SageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Git errors
            SageError::GitNoStagedChanges => {
                write!(f, "No staged changes found\n\nTip: Stage files with 'sage <files>' or use 'sage --all' to stage all changes")
            },
            SageError::GitNoChanges => {
                write!(f, "No changes detected\n\nTip: Make some changes to your files before committing")
            },
            SageError::GitStagingFailed(details) => {
                write!(f, "Failed to stage files: {}\n\nTip: Check that the file paths are correct and accessible", details)
            },
            SageError::GitCommitFailed(details) => {
                write!(f, "Failed to commit changes: {}\n\nTip: Check git status and ensure you have proper git configuration", details)
            },
            SageError::GitPushFailed(details) => {
                write!(f, "Failed to push changes: {}\n\nTip: Check your remote repository settings and network connection", details)
            },
            SageError::GitDiffFailed(details) => {
                write!(f, "Failed to get git diff: {}", details)
            },

            // Config errors
            SageError::ConfigInvalidJson(details) => {
                write!(f, "Invalid configuration file: {}\n\nTip: Delete ~/.sage-config.json and reconfigure", details)
            },
            SageError::ConfigApiKeyNotSet { provider } => {
                write!(f, "API key not set for provider: {}\n\nTip: Run 'sage config -p {} -k <your-api-key>'", provider, provider)
            },
            SageError::ConfigProviderNotFound { provider } => {
                write!(f, "No configuration found for provider: {}\n\nTip: Run 'sage config -p {} -k <your-api-key>' to configure", provider, provider)
            },
            SageError::ConfigProviderNotConfigured { provider } => {
                write!(f, "Provider '{}' not configured\n\nTip: Run 'sage config -p {} -k <your-api-key>' first", provider, provider)
            },
            SageError::ConfigHomeDirNotFound => {
                write!(f, "Could not find home directory\n\nTip: Ensure the HOME environment variable is set")
            },

            // API errors
            SageError::ApiNetworkError { provider, details } => {
                write!(f, "Network error connecting to {}: {}\n\nTip: Check your internet connection and API endpoint availability", provider, details)
            },
            SageError::ApiAuthError { provider } => {
                write!(f, "Authentication failed for {}\n\nTip: Verify your API key with 'sage config -s' and update if needed", provider)
            },
            SageError::ApiResponseError { provider, details } => {
                write!(f, "{} API error: {}\n\nTip: Check the API status page or try again later", provider, details)
            },
            SageError::ApiNoResponse { provider } => {
                write!(f, "No response from {} API\n\nTip: The API may be experiencing issues. Try again later", provider)
            },
            SageError::ApiUnsupportedProvider { provider } => {
                write!(f, "Unsupported provider: {}\n\nTip: Supported providers are: openai, claude", provider)
            },

            // I/O errors
            SageError::IoError(details) => {
                write!(f, "I/O error: {}", details)
            },

            // Editor errors
            SageError::EditorFailed => {
                write!(f, "Failed to open editor\n\nTip: Set your EDITOR environment variable or use a different editor")
            },

            // User input errors
            SageError::InvalidInput(details) => {
                write!(f, "Invalid input: {}", details)
            },
        }
    }
}

impl Error for SageError {}

impl From<io::Error> for SageError {
    fn from(error: io::Error) -> Self {
        SageError::IoError(error.to_string())
    }
}

impl From<reqwest::Error> for SageError {
    fn from(error: reqwest::Error) -> Self {
        SageError::ApiNetworkError {
            provider: "Unknown".to_string(),
            details: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for SageError {
    fn from(error: serde_json::Error) -> Self {
        SageError::ConfigInvalidJson(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SageError>;
