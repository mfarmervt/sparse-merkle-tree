use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub type Hash = [u8; 32];
pub type Key = u64;
pub type Value = u64;

fn hash_leaf(key: Key, value: Value) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(key.to_be_bytes());
    hasher.update(value.to_be_bytes());
    let hash: Hash = hasher.finalize().into();
    return hash;
}

/// Hash two child hashes into their parent hash.
/// (Implementation to be filled in later.)
fn hash_internal(left: Hash, right: Hash) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    let hash: Hash = hasher.finalize().into();
    return hash;
}

///Define some fixed empty leaf representation.
fn hash_empty_leaf() -> Hash {
    let empty = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(empty);
    let empty_leaf_hash = hasher.finalize().into();
    return empty_leaf_hash;
}

///Builds a tree of empty hashes
fn build_empty_hashes(depth: u8) -> Vec<Hash> {
    let d = depth as usize;

    // Allocate [0..=depth]
    let mut empty_hashes = vec![[0u8; 32]; d + 1];

    // Set the empty leaf hash at level = depth
    empty_hashes[d] = hash_empty_leaf();

    // If depth == 0, the leaf is also the root, so we're done
    if d == 0 {
        return empty_hashes;
    }

    // Fill parents from level = depth-1 down to 0
    for level in (0..d).rev() {
        let child = empty_hashes[level + 1];
        let parent = hash_internal(child, child);
        empty_hashes[level] = parent;
    }

    empty_hashes
}

///Struct to store node position in the tree.  Level will be the depth of the tree the node is in, and index will be the
/// node's index within that level
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct NodeId {
    level: u8,
    index: u64,
}

#[derive(Debug)]
pub struct SparseMerkleTree {
    depth: u8,                    // fixed to 64 for u64 keys
    nodes: HashMap<NodeId, Hash>, // only stores non-default hashes
    empty_hashes: Vec<Hash>,      // default hash for each level 0..=depth
}

impl SparseMerkleTree {
    /// Creates an empty Sparse Merkle tree.
    pub fn new() -> Self {
        let depth: u8 = 64;
        let empty_hashes = build_empty_hashes(depth);
        let nodes = HashMap::new();
        SparseMerkleTree {
            depth,
            nodes,
            empty_hashes,
        }
    }

    /// Returns the current root hash.  If the tree is empty it will return empty_hashes[0]
    pub fn root(&self) -> Hash {
        let root_id = NodeId { level: 0, index: 0 };
        *self.nodes.get(&root_id).unwrap_or(&self.empty_hashes[0])
    }

    /// Inserts, updates, or deletes the value for a given key and updates the path to the root
    pub fn update(&mut self, key: Key, value: Option<Value>) {
        let leaf_id = self.compute_leaf_node_id(key); //based on the key, compute the leaf NodeID for that leaf
        let leaf_hash = match value {
            //hash leaf if there is a value
            Some(v) => hash_leaf(key, v),
            None => self.empty_hashes[self.depth as usize], //if there is no value then use empty hash
        };
        self.set_node_hash(leaf_id, leaf_hash);

        //recompute path to root
        let mut child_id = leaf_id;
        while child_id.level > 0 {
            let sibling_index = child_id.index ^ 1;
            let sibling_id = NodeId {
                level: child_id.level,
                index: sibling_index,
            };

            let is_left_child = child_id.index % 2 == 0;
            let (left_id, right_id) = if is_left_child {
                (child_id, sibling_id)
            } else {
                (sibling_id, child_id)
            };

            let left_hash = self.get_node_hash(&left_id);
            let right_hash = self.get_node_hash(&right_id);
            let parent_hash = hash_internal(left_hash, right_hash);

            let parent_id = NodeId {
                level: child_id.level - 1,
                index: child_id.index / 2,
            };

            self.set_node_hash(parent_id, parent_hash);
            child_id = parent_id;
        }
    }

    fn get_node_hash(&self, node_id: &NodeId) -> Hash {
        self.nodes
            .get(node_id)
            .copied()
            .unwrap_or(self.empty_hashes[node_id.level as usize])
    }

    fn set_node_hash(&mut self, node_id: NodeId, hash: Hash) {
        if hash == self.empty_hashes[node_id.level as usize] {
            // Keep storage sparse by removing default hashes.
            self.nodes.remove(&node_id);
        } else {
            self.nodes.insert(node_id, hash);
        }
    }

    ///Tells you exact position in the tree where a node exists by returning the node's NodeID.   
    fn compute_leaf_node_id(&self, key: Key) -> NodeId {
        let mut index: u64 = 0;

        for depth in 0..self.depth {
            let shift = (self.depth - 1 - depth) as u32;
            let bit = (key >> shift) & 1;
            index = (index << 1) | bit;
        }
        NodeId {
            level: self.depth,
            index,
        }
    }
}

fn main() {
    println!("Hello world!");

    let mut tree = SparseMerkleTree::new();
    let root = tree.root();

    let hex: String = root.iter().map(|b| format!("{:02x}", b)).collect();
    println!("Root hash: {}", hex);

    let key = 7u64;
    let value = 8u64;

    tree.update(key, Some(value));

    let new_root = tree.root();
    let hex_2: String = new_root.iter().map(|b| format!("{:02x}", b)).collect();
    println!("New root hash: {}", hex_2);
}
