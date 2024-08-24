use std::collections::HashMap;

use camino::Utf8PathBuf;
use petgraph::{
    dot::{self, Dot},
    Graph,
};

use crate::dir;

pub fn print_graph() {
    let graph = get_graph();

    // Output the tree to `graphviz` `DOT` format
    println!(
        "{:?}",
        Dot::with_config(&graph, &[dot::Config::EdgeNoLabel])
    );
}

fn get_parent(path: &Utf8PathBuf) -> Utf8PathBuf {
    let curr_dir = dir::current_dir();
    let parent = path.parent().unwrap();
    parent.strip_prefix(&curr_dir).unwrap().to_path_buf()
}

pub fn get_graph() -> Graph<Utf8PathBuf, i32> {
    let mut graph: Graph<Utf8PathBuf, i32> = Graph::new();
    // Collection of `file` - `graph index`.
    let mut indices = HashMap::<Utf8PathBuf, _>::new();
    let files = get_all_files_tf_and_hcl_files();
    for f in files {
        let f_parent = get_parent(&f);
        let node_index = graph.add_node(f_parent.clone());
        indices.insert(f_parent.to_path_buf(), node_index);
        let dependencies = get_dependencies(&f);
        for d in dependencies {
            let d_parent = get_parent(&d);
            let existing_index = indices.get(&d_parent.to_path_buf());

            if let Some(&existing_index) = existing_index {
                graph.add_edge(node_index, existing_index, 0);
            } else {
                let d_index = graph.add_node(d_parent.clone());
                indices.insert(d_parent.to_path_buf(), d_index);
                graph.add_edge(node_index, d_index, 0);
            }
        }
    }
    graph
}

/// Get the dependencies of a file
/// Dependencies are anything in the file like `source = "path"` or `config_path = "path"`.
fn get_dependencies(file: &Utf8PathBuf) -> Vec<Utf8PathBuf> {
    let content = std::fs::read_to_string(file).expect("could not read file");
    let mut dependencies = vec![];
    for line in content.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(&first_token) = tokens.first() else {
            continue;
        };
        if first_token != "source" || first_token != "config_path" {
            continue;
        }
        let dependency = tokens[2].trim_matches('"');
        let module_path = file.parent().unwrap().join(dependency);
        dependencies.push(module_path);
    }
    dependencies
}

/// Get all the files that might contain a dependency
pub fn get_all_files_tf_and_hcl_files() -> Vec<Utf8PathBuf> {
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
