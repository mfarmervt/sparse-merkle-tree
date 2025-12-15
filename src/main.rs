/*
This is an implementation of a Sparse Merkle MAP, or a Merkleized key-value store, where each leaf is a hash of the key and 
value of a node added to the tree.  For example, for a node n = (key_bytes, value_bytes) where k = key and v = value, 
the leaf n would be H( key_bytes || value_bytes ).  


1. Goal:
--------------------------------------------------------------
- Implement a Sparse Merkle Tree that acts as a key–value map:
    key:   u64
    value: u64
- Merkle root commits to the entire mapping (key and value).
- Supports:
    - Setting a key to some value
    - Deleting a key (making it "empty")
    - Getting the current root hash
    - (Later) generating & verifying Merkle proofs

2. Key and tree shape
--------------------------------------------------------------

- Keys are unsigned u64 values represented in binary.  
- The 64-bits of the key represent the path from the root to the leaf, where the most significant bit (bit 63)
  is the decision at depth 0 (the root), the next significant bit at depth 1,..., the least significant bit at depth 64
- The tree has fixed depth of 64, where depth 0 is the root and depth 64 are the leaves.
- Conceptually, there will be 2^64 leaves, but we will not store all of these, only the ones that differ from the default
  empty hashes at that level.  

3. Hash type and hashing rules
--------------------------------------------------------------

- Hash will be represented as a 32-byte fixed size array
    type Hash = [u8; 32]
- Hashing primitives:

        fn hash_leaf(key: u64, value: u64) -> Hash
            - serialize key to 8 bytes (big-endian)
            - serialize value to 8 bytes (big-endian)
            - compute: H( key_bytes || value_bytes )

        fn hash_internal(left: Hash, right: Hash) -> Hash
            - compute: H( left || right )

        fn hash_empty_leaf() -> Hash
            - define some fixed representation of an empty leaf, e.g. a fixed byte string
            - compute: H( EMPTY_LEAF_BYTES ) to get a base empty leaf hash

- Default hashes ("empty hashes") per level:

        - We maintain a vector: empty_hashes: Vec<Hash>
        - empty_hashes[level] = the default hash for any node at that level
        - Construction:
            - empty_hashes[64] = hash_empty_leaf()
            - for level in (63 down to 0):
                empty_hashes[level] = hash_internal(
                    empty_hashes[level + 1],
                    empty_hashes[level + 1]
                )

        - A completely empty tree (no keys set) has root = empty_hashes[0].

 3. Node Identification and Storage
--------------------------------------------------------------

    - Each node in the tree is identified by:
        - level: u8   (0 = root, 64 = leaves)
        - index: u64  (position within that level; 0..(2^level - 1))

      We'll represent this conceptually as:

        struct NodeId {
            level: u8,
            index: u64,
        }

    - Internal storage:

        struct SparseMerkleTree {
            depth: u8,                     // fixed to 64 for u64 keys
            nodes: HashMap<NodeId, Hash>,  // only stores non-default hashes
            empty_hashes: Vec<Hash>,       // default hash for each level 0..=depth
        }

        - Invariant:
            - If a node (level, index) is NOT present in `nodes`, then its hash is:
                empty_hashes[level].

            - If a node IS present in `nodes`, then:
                - Its hash != empty_hashes[level].
                - Deleting or resetting it to "empty" means removing it from `nodes`,
                  not storing the default hash explicitly.


    4. Path Derivation from Key
  --------------------------------------------------------------

    - For a given key (u64), we derive the path from root to leaf by reading its bits.

    - View key bits as: b63 b62 ... b1 b0 (b63 = most significant bit).

    - At each depth d in [0..63]:
        - bit = b(63 - d)
        - If bit == 0 → go to the left child
        - If bit == 1 → go to the right child

    - Node indices:
        - Root: (level = 0, index = 0)
        - Children:
            - Left child of (level, index) at next level:   (level + 1, index * 2)
            - Right child of (level, index) at next level:  (level + 1, index * 2 + 1)

    - For a given key, the leaf position is:
        - level = depth (64)
        - index = integer determined by its bit pattern (path).


    5. Public API (Conceptual)
--------------------------------------------------------------

    struct SparseMerkleTree {
        depth: u8,
        nodes: HashMap<NodeId, Hash>,
        empty_hashes: Vec<Hash>,
    }

    Core methods:

    - new(depth: u8) -> SparseMerkleTree
        - Typically called as new(64) for u64 keys.
        - Precomputes empty_hashes[0..=depth].
        - Initializes an empty HashMap for nodes.
        - Initially the tree is "empty": root = empty_hashes[0].

    - root() -> Hash
        - Returns the current root hash of the tree.
        - If there is an explicit entry for NodeId { level: 0, index: 0 } in `nodes`,
          return that hash.
        - Otherwise, return empty_hashes[0].

    - update(key: u64, value: Option<u64>)
        - If value is Some(v):
            - Compute leaf_hash = hash_leaf(key, v).
        - If value is None:
            - The leaf becomes empty; its hash is conceptually empty_hashes[depth].
            - This is equivalent to "deleting" the key from the map.

        - Steps:
            1. Compute the leaf's NodeId from the key:
                - start at root (0, 0)
                - for each bit of the key (from most significant to least),
                  choose left/right and update (level, index) accordingly.
                - After 64 steps, we reach leaf NodeId (depth, leaf_index).

            2. Update the leaf node:
                - If value is Some(v):
                    - Insert or update nodes[leaf_node_id] = leaf_hash.
                - If value is None:
                    - Remove nodes[leaf_node_id] if present (because the default hash
                      for this level is already implied by empty_hashes[depth]).

            3. Walk up from the leaf to the root recomputing hashes:
                - For each parent level from depth - 1 down to 0:
                    - Given a node at (level + 1, child_index), find:
                        - its sibling at the same level:
                            - If child_index is even → sibling_index = child_index + 1
                            - If child_index is odd  → sibling_index = child_index - 1
                        - parent_index at the level above:
                            - parent_index = child_index / 2

                    - For both child and sibling:
                        - Get their hashes:
                            - If they exist in `nodes`, use those.
                            - Otherwise, use empty_hashes[level + 1].

                    - Compute parent_hash = hash_internal(left_child_hash, right_child_hash),
                      where left/right are determined by even/odd index.

                    - Compare parent_hash to empty_hashes[level]:
                        - If parent_hash == empty_hashes[level]:
                            - Remove this parent NodeId from nodes if present.
                        - Else:
                            - Insert or update nodes[parent_node_id] = parent_hash.

                    - Move up to the parent node and repeat until reaching level 0.

        - After update finishes, root() reflects the updated key–value mapping.

Helper: compute leaf NodeId from key

    Define a function inside impl SparseMerkleTree:

    Signature: fn compute_leaf_node_id(&self, key: u64) -> NodeId

        Behavior:

            Start with:

                level = 0

                index = 0

        For each depth step d from 0 to self.depth - 1:

Determine which bit of the key to look at:

Use the bit at position (self.depth - 1 - d) (so for depth 64, start from bit 63 down to 0).

Extract that bit from the key (you’ll use bitwise operations in code, but don’t write them yet).

If bit == 0:

Go to the left child:

index = index * 2

If bit == 1:

Go to the right child:

index = index * 2 + 1

Increment level by 1.

After the loop:

level should equal self.depth.

index is the leaf index.

Return a NodeId with this (level, index).

This is how you map a u64 key to its leaf position.

*/


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
fn hash_empty_leaf() -> Hash{
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


/*
    CORE API
*/

#[derive(Debug)]
pub struct SparseMerkleTree {
    depth: u8,                     // fixed to 64 for u64 keys
    nodes: HashMap<NodeId, Hash>,  // only stores non-default hashes
    empty_hashes: Vec<Hash>,       // default hash for each level 0..=depth
}


impl SparseMerkleTree {
    /// Creates an empty Sparse Merkle tree.
    pub fn new() -> Self {
        unimplemented!()
    }

     /// Returns the current root hash.  If the tree is empty it will return empty_hashes[0]
     pub fn root(&self) -> Hash {
        unimplemented!()
    }

    /// Inserts, updates, or deletes the value for a given key and updates the path to the root
    pub fn update(&mut self, key: Key, value: Option<Value>) {
        unimplemented!()
    }

    fn get_node_hash(&self, node_id: &NodeId) -> Hash{
        unimplemented!()
    }

    fn set_node_hash(&mut self, node_id: NodeId, hash: Hash){
        unimplemented!()
    }
    
    ///Tells you exact position in the tree where a node exists by returning the node's NodeID.   
    fn compute_leaf_node_id(&self, key: Key) -> NodeId{
        unimplemented!()
    }
   
}

fn main(){
    println!("Hello world!")
}


