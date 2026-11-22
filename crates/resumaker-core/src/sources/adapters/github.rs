//! GitHub: `https://api.github.com/users/<name>/repos` becomes a list of projects.

use serde_json::Value;

use super::Adapter;
use crate::profile::Project;
use crate::{Error, Profile};

/// Forks and repositories without a description rarely belong on a resume.
pub struct GitHubRepos;

impl Adapter for GitHubRepos {
    fn name(&self) -> &'static str {
        "github"
    }

    fn matches(&self, url: &str) -> bool {
        let path = url
            .strip_prefix("https://api.github.com/")
            .map(|p| p.split('?').next().unwrap_or(p).trim_end_matches('/'));
        matches!(path, Some(p) if (p.starts_with("users/") || p.starts_with("orgs/")) && p.ends_with("/repos"))
    }

    fn convert(&self, response: &Value) -> Result<Profile, Error> {
        let repos = response
            .as_array()
            .ok_or_else(|| Error::Llm("GitHub response is not a list of repositories".into()))?;
        let text = |r: &Value, k: &str| {
            r.get(k)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let mut projects: Vec<(u64, Project)> = repos
            .iter()
            .filter(|r| r.get("fork").and_then(Value::as_bool) != Some(true))
            .filter(|r| r.get("archived").and_then(Value::as_bool) != Some(true))
            .filter_map(|r| {
                let description = text(r, "description")?;
                let stars = r
                    .get("stargazers_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let mut keywords: Vec<String> = text(r, "language").into_iter().collect();
                keywords.extend(
                    r.get("topics")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned),
                );
                let mut highlights = Vec::new();
                if stars > 0 {
                    highlights.push(format!("{stars} stars on GitHub"));
                }
                Some((
                    stars,
                    Project {
                        name: text(r, "name"),
                        description: Some(description),
                        url: text(r, "homepage").or_else(|| text(r, "html_url")),
                        keywords,
                        highlights,
                        start_date: text(r, "created_at").map(|d| d[..d.len().min(7)].to_owned()),
                        ..Project::default()
                    },
                ))
            })
            .collect();
        projects.sort_by_key(|p| std::cmp::Reverse(p.0));
        Ok(Profile {
            projects: projects.into_iter().map(|(_, p)| p).collect(),
            ..Profile::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn matches_repo_listings_only() {
        let a = GitHubRepos;
        assert!(a.matches("https://api.github.com/users/octocat/repos"));
        assert!(a.matches("https://api.github.com/users/octocat/repos?per_page=100"));
        assert!(!a.matches("https://api.github.com/users/octocat"));
        assert!(!a.matches("https://example.com/users/x/repos"));
    }

    #[test]
    fn converts_repos_to_projects() {
        let response = json!([
            { "name": "small", "description": "A tool", "stargazers_count": 2,
              "html_url": "https://github.com/x/small", "language": "Rust", "created_at": "2021-05-01T00:00:00Z" },
            { "name": "big", "description": "Popular lib", "stargazers_count": 300,
              "html_url": "https://github.com/x/big", "topics": ["slam"] },
            { "name": "fork", "description": "Someone else's", "fork": true },
            { "name": "empty", "description": null }
        ]);
        let p = GitHubRepos.convert(&response).unwrap();
        let names: Vec<_> = p
            .projects
            .iter()
            .filter_map(|p| p.name.as_deref())
            .collect();
        assert_eq!(names, ["big", "small"]);
        assert_eq!(p.projects[1].keywords, ["Rust"]);
        assert_eq!(p.projects[1].start_date.as_deref(), Some("2021-05"));
        assert_eq!(p.projects[0].highlights, ["300 stars on GitHub"]);
    }
}
