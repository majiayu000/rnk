//! Git icons (Nerd Font)

/// Git branch
pub const fn branch() -> &'static str { "" }

/// Git commit
pub const fn commit() -> &'static str { "" }

/// Git merge
pub const fn merge() -> &'static str { "" }

/// Git pull request
pub const fn pull_request() -> &'static str { "" }

/// Git compare
pub const fn compare() -> &'static str { "" }

/// Git stash
pub const fn stash() -> &'static str { "" }

/// Git tag
pub const fn tag() -> &'static str { "" }

/// Git remote
pub const fn remote() -> &'static str { "" }

/// Added file
pub const fn added() -> &'static str { "" }

/// Modified file
pub const fn modified() -> &'static str { "" }

/// Deleted file
pub const fn deleted() -> &'static str { "" }

/// Renamed file
pub const fn renamed() -> &'static str { "" }

/// Untracked file
pub const fn untracked() -> &'static str { "" }

/// Ignored file
pub const fn ignored() -> &'static str { "" }

/// Conflict
pub const fn conflict() -> &'static str { "" }

/// Staged
pub const fn staged() -> &'static str { "" }

/// Unstaged
pub const fn unstaged() -> &'static str { "" }

/// Ahead
pub const fn ahead() -> &'static str { "" }

/// Behind
pub const fn behind() -> &'static str { "" }

/// GitHub
pub const fn github() -> &'static str { "" }

/// GitLab
pub const fn gitlab() -> &'static str { "" }

/// Bitbucket
pub const fn bitbucket() -> &'static str { "" }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_icons() {
        assert_eq!(branch(), "");
        assert_eq!(commit(), "");
        assert_eq!(github(), "");
    }
}
