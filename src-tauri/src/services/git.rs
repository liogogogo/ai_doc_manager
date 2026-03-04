use git2::Repository;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("Not a git repository: {0}")]
    NotARepo(String),
}

pub struct GitService {
    repo: Repository,
}

impl GitService {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = Repository::discover(path)
            .map_err(|_| GitError::NotARepo(path.display().to_string()))?;
        Ok(Self { repo })
    }

    pub fn get_recent_diff(&self, num_commits: usize) -> Result<String, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let oids: Vec<git2::Oid> = revwalk
            .take(num_commits + 1)
            .filter_map(|r| r.ok())
            .collect();

        if oids.len() < 2 {
            return Ok(String::new());
        }

        let new_commit = self.repo.find_commit(oids[0])?;
        let old_commit = self.repo.find_commit(*oids.last().unwrap())?;

        let new_tree = new_commit.tree()?;
        let old_tree = old_commit.tree()?;

        let diff = self.repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;

        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            diff_text.push_str(content);
            true
        })?;

        Ok(diff_text)
    }

    pub fn get_head_hash(&self) -> Result<String, GitError> {
        let head = self.repo.head()?;
        let oid = head.target().ok_or_else(|| {
            GitError::Git(git2::Error::from_str("HEAD has no target"))
        })?;
        Ok(oid.to_string()[..7].to_string())
    }
}
