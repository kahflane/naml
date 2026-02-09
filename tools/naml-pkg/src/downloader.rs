///
/// Git repository downloader for naml package manager.
///
/// Handles cloning Git repositories and checking out specific references
/// (tags, branches, or commit revisions) using the git2 crate.
///
/// Repositories are cached locally — if the destination already exists,
/// the download is skipped to avoid redundant network operations.
///

use std::path::Path;
use git2::Repository;
use crate::errors::PackageError;
use crate::manifest::GitRef;

pub fn download_git_package(url: &str, git_ref: &GitRef, dest: &Path) -> Result<(), PackageError> {
    if dest.exists() && dest.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Ok(());
    }

    let repo = Repository::clone(url, dest).map_err(|e| PackageError::GitCloneFailed {
        url: url.to_string(),
        reason: e.message().to_string(),
    })?;

    checkout_ref(&repo, git_ref)?;

    Ok(())
}

pub fn checkout_ref(repo: &Repository, git_ref: &GitRef) -> Result<(), PackageError> {
    match git_ref {
        GitRef::Default => Ok(()),

        GitRef::Tag(tag) => {
            let reference = repo
                .find_reference(&format!("refs/tags/{}", tag))
                .map_err(|e| PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: tag.clone(),
                    reason: e.message().to_string(),
                })?;

            let commit = reference.peel_to_commit().map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: tag.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            repo.checkout_tree(commit.as_object(), None).map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: tag.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            repo.set_head_detached(commit.id()).map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: tag.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            Ok(())
        }

        GitRef::Branch(branch) => {
            let reference = repo
                .find_reference(&format!("refs/remotes/origin/{}", branch))
                .map_err(|e| PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: branch.clone(),
                    reason: e.message().to_string(),
                })?;

            let commit = reference.peel_to_commit().map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: branch.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            repo.checkout_tree(commit.as_object(), None).map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: branch.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            repo.set_head_detached(commit.id()).map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: branch.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            Ok(())
        }

        GitRef::Rev(rev) => {
            let oid = git2::Oid::from_str(rev).map_err(|e| PackageError::GitCheckoutFailed {
                url: get_repo_url(repo),
                reference: rev.clone(),
                reason: e.message().to_string(),
            })?;

            let commit = repo.find_commit(oid).map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: rev.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            repo.checkout_tree(commit.as_object(), None).map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: rev.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            repo.set_head_detached(oid).map_err(|e| {
                PackageError::GitCheckoutFailed {
                    url: get_repo_url(repo),
                    reference: rev.clone(),
                    reason: e.message().to_string(),
                }
            })?;

            Ok(())
        }
    }
}

fn get_repo_url(repo: &Repository) -> String {
    repo.find_remote("origin")
        .ok()
        .and_then(|remote| remote.url().map(String::from))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_download_skips_cached() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("cached-package");

        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("dummy.txt"), "cached content").unwrap();

        let result = download_git_package(
            "https://github.com/example/repo",
            &GitRef::Default,
            &dest,
        );

        assert!(result.is_ok());
    }
}
