use std::collections::{BTreeSet, HashMap};

use camino::{Utf8Path, Utf8PathBuf};
use tracing::{debug, warn};

use petgraph::{graph::NodeIndex, visit::EdgeRef as _, Graph};

use crate::{dir, LOCKFILE};

/// A graph of terragrunt and terraform modules.
pub struct ModulesGraph {
    pub graph: Graph<Utf8PathBuf, i32>,
}

impl ModulesGraph {
    pub fn new(outdated_packages: Option<&BTreeSet<Utf8PathBuf>>) -> Self {
        let mut graph: Graph<Utf8PathBuf, i32> = Graph::new();
        // Collection of `file` - `graph index`.
        let mut indices = HashMap::<Utf8PathBuf, NodeIndex>::new();
        let files = get_all_tf_and_hcl_files();
        for f in files {
            let f_parent = dir::get_stripped_parent(&f);
            let node_index = indices
                .get(&f_parent)
                .cloned()
                .unwrap_or_else(|| add_node(&mut graph, f_parent, &mut indices, outdated_packages));
            let dependencies = get_dependencies(&f);
            for d in dependencies {
                let d_index = indices
                    .get(&d)
                    .cloned()
                    .unwrap_or_else(|| add_node(&mut graph, d, &mut indices, outdated_packages));

                graph.update_edge(node_index, d_index, 0);
            }
        }
        Self { graph }
    }
    pub fn get_dependent_modules<T>(&self, modules: &[T]) -> Vec<Utf8PathBuf>
    where
        T: AsRef<Utf8Path>,
    {
        let modules = modules.iter().map(|m| m.as_ref()).collect::<Vec<_>>();
        let mut dependent_modules = vec![];
        for m in modules {
            let dependent_modules_of_dir = self.get_dependent_modules_of_dir(m);
            dependent_modules.extend(dependent_modules_of_dir);
        }
        dependent_modules
    }

    pub fn get_dependent_modules_of_dir(&self, module: &Utf8Path) -> Vec<Utf8PathBuf> {
        let module_index = self
            .graph
            .node_indices()
            .find(|i| self.graph[*i] == module)
            .expect("module not found in graph");
        let mut dependent_modules = vec![];
        for edge in self
            .graph
            .edges_directed(module_index, petgraph::Direction::Incoming)
        {
            let dependent_module_index = edge.source();
            dependent_modules.push(self.graph[dependent_module_index].clone());
        }
        dependent_modules
    }
}

fn add_node(
    graph: &mut Graph<Utf8PathBuf, i32>,
    dir: Utf8PathBuf,
    indices: &mut HashMap<Utf8PathBuf, NodeIndex>,
    outdated_packages: Option<&BTreeSet<Utf8PathBuf>>,
) -> NodeIndex {
    let label = if let Some(outdated_packages) = outdated_packages {
        // add an emoji to the path just for the graph visualization.
        if outdated_packages.contains(&dir) {
            // the module isn't up-to-date and it needs to be updated.
            dir.join(" ❌")
        } else if dir.join(LOCKFILE).exists() {
            // the module isn't in the outdated packages and it contains a lockfile, so it's up-to-date
            dir.join(" ✅")
        } else {
            // The module doesn't contain a lockfile, so we don't need to update it.
            dir.clone()
        }
    } else {
        dir.clone()
    };
    debug!("Adding node: {:?}", label);
    let node_index = graph.add_node(label.clone());
    indices.insert(dir, node_index);
    node_index
}

/// Get the dependencies of a file
/// Dependencies are anything in the file like `source = "path"` or `config_path = "path"`.
fn get_dependencies(file: &Utf8Path) -> Vec<Utf8PathBuf> {
    let content = std::fs::read_to_string(file).expect("could not read file");
    let mut dependencies = vec![];
    for line in content.lines() {
        if let Some(dependency) = get_dependency_from_line(line) {
            let module_path = file.parent().unwrap().join(dependency);
            let relative_path = get_relative_path(&module_path);
            debug!("found dependency {:?} from line {line}", relative_path);
            dependencies.push(relative_path);
        }
    }
    dependencies
}

/// Get all the files that might contain a dependency
pub fn get_all_tf_and_hcl_files() -> Vec<Utf8PathBuf> {
    let mut files = vec![];
    let current_dir = dir::current_dir();
    let walker = ignore::WalkBuilder::new(current_dir)
        // Read hidden files
        .hidden(false)
        .build();

    for entry in walker {
        let entry = entry.expect("invalid entry");
        let file_type = entry.file_type().expect("unknown file type");
        if !file_type.is_dir()
            && (entry.path().extension() == Some("tf".as_ref())
                || entry.path().extension() == Some("hcl".as_ref()))
        {
            let path = entry.path().to_path_buf();
            let utf8path = Utf8PathBuf::from_path_buf(path).unwrap();
            files.push(utf8path);
        }
    }
    files
}

fn get_dependency_from_line(line: &str) -> Option<&str> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let first_token = *tokens.first()?;
    if first_token != "source" && first_token != "config_path" {
        return None;
    }
    let second_token = *tokens.get(1)?;
    if second_token != "=" {
        return None;
    }
    let third_token = tokens[2].trim_matches('"');
    let dependency = third_token
        .trim_start_matches("git::")
        .split('?')
        .next()
        .unwrap_or(third_token);
    if !dependency.starts_with(".") {
        // it's not a directory. E.g. it's `source  = "hashicorp/aws"`.
        return None;
    }

    Some(dependency)
}

fn get_relative_path(path: &Utf8Path) -> Utf8PathBuf {
    // canonicalize to convert `a/b/../c` to `a/c`
    let canonicalized = match path.canonicalize_utf8() {
        Ok(c) => c,
        Err(err) => {
            warn!("Could not canonicalize path {path}: {err:?}");
            path.to_path_buf()
        }
    };
    dir::strip_current_dir(&canonicalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino_tempfile::NamedUtf8TempFile;

    #[test]
    fn dependencies_are_read() {
        let file = NamedUtf8TempFile::new().unwrap();
        let content = r#"
                        source = "../aaaa"
                "#;
        fs_err::write(file.path(), content).unwrap();
        let dependencies = get_dependencies(file.path());
        assert_eq!(dependencies.len(), 1);
    }
}
