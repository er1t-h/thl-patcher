use std::{
    cell::{LazyCell, OnceCell},
    collections::{BTreeSet, HashMap},
    fs::File,
    io::BufReader,
    path::Path,
};

use either::Either;
use petgraph::{graph::{DiGraph, NodeIndex}, visit::DfsEvent};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Clone)]
pub struct Determinants {
    pub file: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JumpingUpdate {
    pub to: String,
    pub link: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Version {
    pub name: String,
    #[serde(with = "either::serde_untagged_optional")]
    pub update_link: Option<Either<String, Vec<JumpingUpdate>>>,
    pub determinants: Vec<Determinants>,
}

pub struct VersionPath {
    pub index: usize,
    pub update_link: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Source {
    pub versions: Vec<Version>,
    #[serde(skip)]
    digraph: Option<DiGraph<(), u8>>,
}

#[derive(Debug)]
pub enum VersionPathError {}

impl Source {
    pub fn get_current_version(&self, path: &Path) -> Result<Option<usize>, std::io::Error> {
        let mut already_calculated: HashMap<&str, [u8; 64]> = HashMap::new();
        'version: for (i, version) in self.versions.iter().enumerate().rev() {
            tracing::trace!("checking version `{}`", version.name);
            for determinant in &version.determinants {
                let hash = if let Some(x) = already_calculated.get(determinant.file.as_str()) {
                    x
                } else {
                    let mut hasher = Sha256::new();
                    let path = path.join(&determinant.file);
                    let file = match File::open(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            tracing::trace!(
                                "error while opening file `{}`: `{e}`",
                                determinant.file
                            );
                            continue 'version;
                        }
                    };
                    std::io::copy(&mut BufReader::new(file), &mut hasher)?;
                    let mut buffer = [0; 64];
                    match base16ct::lower::encode(&hasher.finalize(), &mut buffer) {
                        Ok(_) => (),
                        Err(e) => unreachable!("64-byte should always be enough: {e}"),
                    }
                    already_calculated
                        .entry(&determinant.file)
                        .or_insert(buffer)
                };

                if hash != determinant.sha256.as_bytes() {
                    tracing::trace!("sha256 mismatch for file `{}`", determinant.file);
                    continue 'version;
                }
            }

            return Ok(Some(i));
        }
        Ok(None)
    }

    pub fn get_versions_to_install(&self, current: usize) -> &[Version] {
        let versions_to_install = self
            .versions
            .split_at_checked(current)
            .unwrap_or_default()
            .1;

        versions_to_install
            .iter()
            .position(|x| x.update_link.is_none())
            .map_or(versions_to_install, |pos| {
                versions_to_install.split_at(pos).0
            })
    }

    pub fn create_digraph(&mut self) -> &DiGraph<(), u8> {
        let mut digraph = DiGraph::new();
        let name_to_index: HashMap<_, _> = self
            .versions
            .iter()
            .map(|r| (r.name.as_str(), digraph.add_node(())))
            .collect();
        for (i, version) in self.versions.iter().enumerate() {
            match &version.update_link {
                Some(Either::Left(_)) => {
                    digraph.add_edge(NodeIndex::new(i), NodeIndex::new(i + 1), 1);
                }
                Some(Either::Right(jumping_updates)) => {
                    for update in jumping_updates {
                        digraph.add_edge(
                            NodeIndex::new(i),
                            name_to_index[update.to.as_str()],
                            1,
                        );
                    }
                }
                None => (),
            };
        }
        self.digraph = Some(digraph);
        self.digraph.as_ref().unwrap()
    }

    pub fn get_path(&mut self, from: usize, to: usize) -> Result<Vec<VersionPath>, VersionPathError> {
        let digraph = self.create_digraph();
        let (_, path) = petgraph::algo::astar(
            digraph,
            NodeIndex::new(from),
            |n| n == NodeIndex::new(to),
            |e| *e.weight(),
            |_| 0,
        )
        .unwrap();
        eprintln!("{:?}", path);
        let mut link_path = Vec::new();
        for [old, new] in path.windows(2).map(|indexes| {
            let old = indexes[0].index();
            let new = indexes[1].index();
            [(&self.versions[old], old), (&self.versions[new], new)]
        }) {
            match old.0.update_link {
                Some(Either::Left(ref l)) => link_path.push(VersionPath { update_link: Some(l.clone()), index: old.1 }),
                Some(Either::Right(ref list)) => {
                    let update = list.iter().find(|x| x.to == new.0.name).unwrap();
                    link_path.push(VersionPath { index: old.1, update_link: Some(update.link.clone()) });
                }
                None => panic!()
            }
        }
        link_path.push(VersionPath { index: path.last().unwrap().index(), update_link: None });
        Ok(link_path)
    }

    pub fn get_reachable_versions(&mut self, current: usize) -> BTreeSet<usize> {
        let digraph = self.create_digraph();
        let mut set = BTreeSet::new();
        petgraph::visit::depth_first_search(&digraph, Some(NodeIndex::new(current)), |event| {
            if let DfsEvent::Discover(start, _) = event {
                set.insert(start.index());
            }
        });
        set.remove(&current);
        set
    }
}
