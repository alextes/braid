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
    NotInitialized = 17,
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

    #[error("braid not initialized\n\nrun `brd init` to set up issue tracking")]
    NotInitialized,

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
            BrdError::NotInitialized => ExitCode::NotInitialized,
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
            BrdError::NotInitialized => "not_initialized",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_values() {
        assert_eq!(i32::from(ExitCode::Success), 0);
        assert_eq!(i32::from(ExitCode::UsageError), 2);
        assert_eq!(i32::from(ExitCode::NotGitRepo), 10);
        assert_eq!(i32::from(ExitCode::ParseError), 16);
    }

    #[test]
    fn test_brd_error_exit_code_mapping() {
        assert_eq!(BrdError::NotGitRepo.exit_code(), ExitCode::NotGitRepo);
        assert_eq!(
            BrdError::ControlRootInvalid("bad".into()).exit_code(),
            ExitCode::ControlRootInvalid
        );
        assert_eq!(
            BrdError::IssueNotFound("brd-1234".into()).exit_code(),
            ExitCode::IssueNotFound
        );
        assert_eq!(
            BrdError::AmbiguousId("brd-".into(), vec!["brd-a".into()]).exit_code(),
            ExitCode::AmbiguousId
        );
        assert_eq!(
            BrdError::ClaimConflict("brd-a".into(), "agent".into()).exit_code(),
            ExitCode::ClaimConflict
        );
        assert_eq!(BrdError::InvalidGraph.exit_code(), ExitCode::InvalidGraph);
        assert_eq!(
            BrdError::ParseError("issue".into(), "bad".into()).exit_code(),
            ExitCode::ParseError
        );
        assert_eq!(
            BrdError::Io(std::io::Error::other("io")).exit_code(),
            ExitCode::GenericFailure
        );
        assert_eq!(
            BrdError::Other("oops".into()).exit_code(),
            ExitCode::GenericFailure
        );
    }

    #[test]
    fn test_brd_error_code_str_mapping() {
        assert_eq!(BrdError::NotGitRepo.code_str(), "not_git_repo");
        assert_eq!(
            BrdError::ControlRootInvalid("bad".into()).code_str(),
            "control_root_invalid"
        );
        assert_eq!(
            BrdError::IssueNotFound("brd-1234".into()).code_str(),
            "issue_not_found"
        );
        assert_eq!(
            BrdError::AmbiguousId("brd-".into(), vec!["brd-a".into()]).code_str(),
            "ambiguous_id"
        );
        assert_eq!(
            BrdError::ClaimConflict("brd-a".into(), "agent".into()).code_str(),
            "claim_conflict"
        );
        assert_eq!(BrdError::InvalidGraph.code_str(), "invalid_graph");
        assert_eq!(
            BrdError::ParseError("issue".into(), "bad".into()).code_str(),
            "parse_error"
        );
        assert_eq!(
            BrdError::Io(std::io::Error::other("io")).code_str(),
            "io_error"
        );
        assert_eq!(BrdError::Other("oops".into()).code_str(), "error");
    }
}
