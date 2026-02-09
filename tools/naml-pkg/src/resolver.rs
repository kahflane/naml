///
/// # Dependency Resolution
///
/// Builds a complete dependency graph from a root manifest by recursively
/// downloading and parsing transitive dependencies. Supports:
///
/// - **Transitive resolution**: Package A depends on B, B depends on C — all get resolved
/// - **Cycle detection**: DFS-based detection with clear error messages showing the cycle path
/// - **Diamond deduplication**: If A->B and A->C both depend on D, D is downloaded once
/// - **Topological ordering**: Returns packages in a safe processing order
///
/// ## Algorithm
///
/// 1. Parse the root manifest's direct dependencies
/// 2. For each dependency, compute its cache path and download if needed
/// 3. Check if the downloaded package has its own naml.toml
/// 4. If so, parse it and recursively resolve its dependencies
/// 5. Track visited packages to detect cycles and avoid duplicates
///

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::cache::{local_package_path, package_cache_path};
use crate::downloader::download_git_package;
use crate::errors::PackageError;
use crate::manifest::{parse_manifest, DependencySource, Manifest};

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub source: DependencySource,
    pub cache_path: PathBuf,
    pub manifest: Option<Manifest>,
}

#[derive(Debug)]
pub struct DependencyGraph {
    pub packages: HashMap<String, ResolvedPackage>,
    pub edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    fn new() -> Self {
        Self {
            packages: HashMap::new(),
            edges: HashMap::new(),
        }
    }
}

pub fn resolve(manifest: &Manifest, manifest_dir: &Path) -> Result<DependencyGraph, PackageError> {
    let mut graph = DependencyGraph::new();
    let mut visiting = HashSet::new();
    let mut path_stack = Vec::new();

    let deps = manifest.dependencies()?;
    for dep in &deps {
        resolve_recursive(
            &dep.name,
            &dep.source,
            manifest_dir,
            &mut graph,
            &mut visiting,
            &mut path_stack,
        )?;
    }

    Ok(graph)
}

fn resolve_recursive(
    name: &str,
    source: &DependencySource,
    manifest_dir: &Path,
    graph: &mut DependencyGraph,
    visiting: &mut HashSet<String>,
    path_stack: &mut Vec<String>,
) -> Result<(), PackageError> {
    if graph.packages.contains_key(name) {
        return Ok(());
    }

    if visiting.contains(name) {
        path_stack.push(name.to_string());
        let cycle_start = path_stack.iter().position(|n| n == name).unwrap();
        let cycle = path_stack[cycle_start..].to_vec();
        return Err(PackageError::CircularDependency { cycle });
    }

    visiting.insert(name.to_string());
    path_stack.push(name.to_string());

    let cache_path = resolve_source(name, source, manifest_dir)?;

    let sub_manifest_path = cache_path.join("naml.toml");
    let sub_manifest = if sub_manifest_path.exists() {
        Some(parse_manifest(&sub_manifest_path)?)
    } else {
        None
    };

    let mut dep_names = Vec::new();

    if let Some(ref sub_m) = sub_manifest {
        let sub_deps = sub_m.dependencies()?;
        let sub_dir = &cache_path;

        for sub_dep in &sub_deps {
            dep_names.push(sub_dep.name.clone());
            resolve_recursive(
                &sub_dep.name,
                &sub_dep.source,
                sub_dir,
                graph,
                visiting,
                path_stack,
            )?;
        }
    }

    graph.edges.insert(name.to_string(), dep_names);
    graph.packages.insert(
        name.to_string(),
        ResolvedPackage {
            name: name.to_string(),
            source: source.clone(),
            cache_path,
            manifest: sub_manifest,
        },
    );

    path_stack.pop();
    visiting.remove(name);

    Ok(())
}

fn resolve_source(
    name: &str,
    source: &DependencySource,
    manifest_dir: &Path,
) -> Result<PathBuf, PackageError> {
    match source {
        DependencySource::Git { url, git_ref } => {
            let dest = package_cache_path(name, url)?;
            download_git_package(url, git_ref, &dest)?;
            Ok(dest)
        }
        DependencySource::Local { path } => {
            let resolved = local_package_path(manifest_dir, &path.to_string_lossy());
            if !resolved.exists() {
                return Err(PackageError::PackageNotFound {
                    name: name.to_string(),
                });
            }
            Ok(resolved)
        }
    }
}

pub fn topological_order(graph: &DependencyGraph) -> Result<Vec<String>, PackageError> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    let mut temp_marks = HashSet::new();

    for name in graph.packages.keys() {
        if !visited.contains(name) {
            topo_visit(name, graph, &mut visited, &mut temp_marks, &mut order)?;
        }
    }

    Ok(order)
}

fn topo_visit(
    name: &str,
    graph: &DependencyGraph,
    visited: &mut HashSet<String>,
    temp_marks: &mut HashSet<String>,
    order: &mut Vec<String>,
) -> Result<(), PackageError> {
    if visited.contains(name) {
        return Ok(());
    }

    if temp_marks.contains(name) {
        return Err(PackageError::CircularDependency {
            cycle: vec![name.to_string()],
        });
    }

    temp_marks.insert(name.to_string());

    if let Some(deps) = graph.edges.get(name) {
        for dep in deps {
            topo_visit(dep, graph, visited, temp_marks, order)?;
        }
    }

    temp_marks.remove(name);
    visited.insert(name.to_string());
    order.push(name.to_string());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_dependency_graph() {
        let graph = DependencyGraph::new();
        let order = topological_order(&graph).unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_topological_order_linear() {
        let mut graph = DependencyGraph::new();

        graph.packages.insert(
            "a".to_string(),
            ResolvedPackage {
                name: "a".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/a"),
                },
                cache_path: PathBuf::from("/tmp/a"),
                manifest: None,
            },
        );
        graph.packages.insert(
            "b".to_string(),
            ResolvedPackage {
                name: "b".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/b"),
                },
                cache_path: PathBuf::from("/tmp/b"),
                manifest: None,
            },
        );

        graph
            .edges
            .insert("a".to_string(), vec!["b".to_string()]);
        graph.edges.insert("b".to_string(), vec![]);

        let order = topological_order(&graph).unwrap();
        let a_pos = order.iter().position(|n| n == "a").unwrap();
        let b_pos = order.iter().position(|n| n == "b").unwrap();
        assert!(b_pos < a_pos, "b should come before a in topological order");
    }

    #[test]
    fn test_topological_order_diamond() {
        let mut graph = DependencyGraph::new();

        graph.packages.insert(
            "a".to_string(),
            ResolvedPackage {
                name: "a".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/a"),
                },
                cache_path: PathBuf::from("/tmp/a"),
                manifest: None,
            },
        );
        graph.packages.insert(
            "b".to_string(),
            ResolvedPackage {
                name: "b".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/b"),
                },
                cache_path: PathBuf::from("/tmp/b"),
                manifest: None,
            },
        );
        graph.packages.insert(
            "c".to_string(),
            ResolvedPackage {
                name: "c".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/c"),
                },
                cache_path: PathBuf::from("/tmp/c"),
                manifest: None,
            },
        );
        graph.packages.insert(
            "d".to_string(),
            ResolvedPackage {
                name: "d".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/d"),
                },
                cache_path: PathBuf::from("/tmp/d"),
                manifest: None,
            },
        );

        graph.edges.insert("a".to_string(), vec!["b".to_string(), "c".to_string()]);
        graph.edges.insert("b".to_string(), vec!["d".to_string()]);
        graph.edges.insert("c".to_string(), vec!["d".to_string()]);
        graph.edges.insert("d".to_string(), vec![]);

        let order = topological_order(&graph).unwrap();
        let a_pos = order.iter().position(|n| n == "a").unwrap();
        let b_pos = order.iter().position(|n| n == "b").unwrap();
        let c_pos = order.iter().position(|n| n == "c").unwrap();
        let d_pos = order.iter().position(|n| n == "d").unwrap();

        assert!(d_pos < b_pos, "d should come before b in topological order");
        assert!(d_pos < c_pos, "d should come before c in topological order");
        assert!(b_pos < a_pos, "b should come before a in topological order");
        assert!(c_pos < a_pos, "c should come before a in topological order");
    }

    #[test]
    fn test_topological_order_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph.packages.insert(
            "a".to_string(),
            ResolvedPackage {
                name: "a".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/a"),
                },
                cache_path: PathBuf::from("/tmp/a"),
                manifest: None,
            },
        );
        graph.packages.insert(
            "b".to_string(),
            ResolvedPackage {
                name: "b".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/b"),
                },
                cache_path: PathBuf::from("/tmp/b"),
                manifest: None,
            },
        );

        graph.edges.insert("a".to_string(), vec!["b".to_string()]);
        graph.edges.insert("b".to_string(), vec!["a".to_string()]);

        let result = topological_order(&graph);
        assert!(result.is_err());
        match result {
            Err(PackageError::CircularDependency { .. }) => {},
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_topological_order_independent() {
        let mut graph = DependencyGraph::new();

        graph.packages.insert(
            "a".to_string(),
            ResolvedPackage {
                name: "a".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/a"),
                },
                cache_path: PathBuf::from("/tmp/a"),
                manifest: None,
            },
        );
        graph.packages.insert(
            "b".to_string(),
            ResolvedPackage {
                name: "b".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/b"),
                },
                cache_path: PathBuf::from("/tmp/b"),
                manifest: None,
            },
        );
        graph.packages.insert(
            "c".to_string(),
            ResolvedPackage {
                name: "c".to_string(),
                source: DependencySource::Local {
                    path: PathBuf::from("/tmp/c"),
                },
                cache_path: PathBuf::from("/tmp/c"),
                manifest: None,
            },
        );

        graph.edges.insert("a".to_string(), vec![]);
        graph.edges.insert("b".to_string(), vec![]);
        graph.edges.insert("c".to_string(), vec![]);

        let order = topological_order(&graph).unwrap();
        assert_eq!(order.len(), 3);
        assert!(order.contains(&"a".to_string()));
        assert!(order.contains(&"b".to_string()));
        assert!(order.contains(&"c".to_string()));
    }
}
