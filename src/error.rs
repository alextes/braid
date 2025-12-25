use thiserror::Error;

/// stable exit codes for automation (per spec section 13)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    GenericFailure = 1,
    UsageError = 2,
    NotGitRepo = 10,
    ControlRootInvalid = 11,
    IssueNotFound = 12,
    AmbiguousId = 13,
    ClaimConflict = 14,
    InvalidGraph = 15,
    ParseError = 16,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}

#[derive(Error, Debug)]
pub enum BrdError {
    #[error("not a git repository")]
    NotGitRepo,

    #[error("control root not found or invalid: {0}")]
    ControlRootInvalid(String),

    #[error("issue not found: {0}")]
    IssueNotFound(String),

    #[error("ambiguous issue id '{0}': matches {1:?}")]
    AmbiguousId(String, Vec<String>),

    #[error("claim conflict: issue '{0}' is claimed by '{1}'")]
    ClaimConflict(String, String),

    #[error("invalid graph: cycle detected")]
    InvalidGraph,

    #[error("parse error in {0}: {1}")]
    ParseError(String, String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl BrdError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            BrdError::NotGitRepo => ExitCode::NotGitRepo,
            BrdError::ControlRootInvalid(_) => ExitCode::ControlRootInvalid,
            BrdError::IssueNotFound(_) => ExitCode::IssueNotFound,
            BrdError::AmbiguousId(_, _) => ExitCode::AmbiguousId,
            BrdError::ClaimConflict(_, _) => ExitCode::ClaimConflict,
            BrdError::InvalidGraph => ExitCode::InvalidGraph,
            BrdError::ParseError(_, _) => ExitCode::ParseError,
            BrdError::Io(_) => ExitCode::GenericFailure,
            BrdError::Other(_) => ExitCode::GenericFailure,
        }
    }

    pub fn code_str(&self) -> &'static str {
        match self {
            BrdError::NotGitRepo => "not_git_repo",
            BrdError::ControlRootInvalid(_) => "control_root_invalid",
            BrdError::IssueNotFound(_) => "issue_not_found",
            BrdError::AmbiguousId(_, _) => "ambiguous_id",
            BrdError::ClaimConflict(_, _) => "claim_conflict",
            BrdError::InvalidGraph => "invalid_graph",
            BrdError::ParseError(_, _) => "parse_error",
            BrdError::Io(_) => "io_error",
            BrdError::Other(_) => "error",
        }
    }
}

pub type Result<T> = std::result::Result<T, BrdError>;
