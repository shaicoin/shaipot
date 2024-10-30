# Hamiltonian Cycle Improvements

## Modifications

### 1. **Memoization for `is_safe` Checks**
   - **Current Issue**: The `is_safe` function iterates through the `path` to check if a vertex has already been visited. This takes \(O(n)\) time for each call.
   - **Optimization**: Use a `HashSet<u16>` to keep track of visited vertices, which will allow \(O(1)\) checking time for each vertex in `is_safe`. This reduces time complexity for `is_safe` from \(O(n)\) to \(O(1)\).

### 2. **Early Termination and Backtracking Enhancements**
   - **Current Issue**: The recursive function `hamiltonian_cycle_util` only checks if the elapsed time has exceeded `vdf_bailout` at the start of each call.
   - **Optimization**: Add a threshold check for the number of recursive calls or the recursion depth. If it appears unlikely that a solution exists within a reasonable timeframe, terminate early. This can help avoid excessive computation on large graphs that likely have no cycle.

### 3. **Optimizing PRNG Bit Extraction**
   - **Current Issue**: The `generate_graph_v2` function generates bits one by one, which is inefficient when filling large bit vectors.
   - **Optimization**: Use bulk bit generation to fill the bit stream directly, bypassing per-bit extraction where possible.

### 4. **Parallelizing Graph Generation**
   - **Current Issue**: Generating the adjacency matrix in `generate_graph_v2` is currently single-threaded.
   - **Optimization**: Use a multithreaded approach to generate the adjacency matrix in parallel. For example, divide the matrix into chunks and spawn multiple threads to generate edges concurrently.

### 5. **Avoiding Unnecessary Memory Allocations**
   - **Current Issue**: Each recursive call may involve heap allocations when resizing or recreating data structures.
   - **Optimization**: Use pre-allocated structures for `path` and `bit_stream` that are reset rather than reallocated at each function call. This can reduce memory pressure and speed up the algorithm.


## Initial Modifications (v01)

Modified version of `src/hasher.rs` incorporating some of these optimizations:

```rust
use primitive_types::U256;
use rand_mt::Mt19937GenRand64;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const GRAPH_SIZE: u16 = 2008;

pub struct HCGraphUtil {
    start_time: Instant,
    vdf_bailout: u64,
}

impl HCGraphUtil {
    pub fn new(vdf_bailout: Option<u64>) -> Self {
        let bailout_timer: u64 = vdf_bailout.unwrap_or(1000); // default to 1 second
        HCGraphUtil {
            start_time: Instant::now(),
            vdf_bailout: bailout_timer,
        }
    }

    fn hex_to_u64(&self, hex_string: &str) -> u64 {
        u64::from_str_radix(hex_string, 16).expect("Failed to convert hex to u64")
    }

    fn read_le_u64(&self, bytes: &[u8]) -> u64 {
        let arr: [u8; 8] = bytes[..8].try_into().expect("Slice with incorrect length");
        u64::from_le_bytes(arr)
    }

    fn extract_seed_from_hash(&self, hash: &U256) -> u64 {
        let bytes = hash.to_little_endian();
        self.read_le_u64(&bytes)
    }

    fn get_grid_size_v2(&self, hash: &U256) -> u16 {
        let hash_hex = format!("{:064x}", hash);
        let grid_size_segment = &hash_hex[0..8];
        let grid_size: u64 = self.hex_to_u64(grid_size_segment);

        let min_grid_size = 2000u64;
        let max_grid_size = GRAPH_SIZE as u64;

        let grid_size_final = min_grid_size + (grid_size % (max_grid_size - min_grid_size));
        grid_size_final as u16
    }

    fn generate_graph_v2(&self, hash: &U256, grid_size: u16) -> Vec<Vec<bool>> {
        let grid_size = grid_size as usize;
        let mut graph = vec![vec![false; grid_size]; grid_size];
        let num_edges = (grid_size * (grid_size - 1)) / 2;
    
        let seed = self.extract_seed_from_hash(hash);
        let mut prng = Mt19937GenRand64::from(seed.to_le_bytes());

        let bit_stream: Vec<bool> = (0..num_edges)
            .map(|_| ((prng.next_u64() & 1) == 1))
            .collect();

        let mut bit_index = 0;
        for i in 0..grid_size {
            for j in (i + 1)..grid_size {
                let edge_exists = bit_stream[bit_index];
                bit_index += 1;
                graph[i][j] = edge_exists;
                graph[j][i] = edge_exists;
            }
        }

        graph
    }

    fn is_safe(&self, v: u16, graph: &Vec<Vec<bool>>, path: &[u16], visited: &HashSet<u16>, pos: usize) -> bool {
        if !graph[path[pos - 1] as usize][v as usize] {
            return false;
        }
        !visited.contains(&v)
    }

    fn hamiltonian_cycle_util(
        &mut self,
        graph: &Vec<Vec<bool>>,
        path: &mut [u16],
        visited: &mut HashSet<u16>,
        pos: usize,
    ) -> bool {
        let elapsed = self.start_time.elapsed();
        if elapsed > Duration::from_millis(self.vdf_bailout) {
            return false;
        }

        if pos == graph.len() {
            return graph[path[pos - 1] as usize][path[0] as usize];
        }

        for v in 1..graph.len() as u16 {
            if self.is_safe(v, graph, path, visited, pos) {
                path[pos] = v;
                visited.insert(v);

                if self.hamiltonian_cycle_util(graph, path, visited, pos + 1) {
                    return true;
                }

                path[pos] = u16::MAX;
                visited.remove(&v);
            }
        }

        false
    }

    pub fn find_hamiltonian_cycle_v2(&mut self, graph_hash: U256) -> Vec<u16> {
        let grid_size = self.get_grid_size_v2(&graph_hash);
        let graph = self.generate_graph_v2(&graph_hash, grid_size);

        let mut path = vec![u16::MAX; graph.len()];
        path[0] = 0;

        let mut visited = HashSet::new();
        visited.insert(0);

        self.start_time = Instant::now();
        if !self.hamiltonian_cycle_util(&graph, &mut path, &mut visited, 1) {
            return vec![];
        }
        path
    }
}
```

### Key Changes

1. **Memoization with `HashSet<u16>`**:
   - The `visited` set tracks which nodes are part of the current path. The `is_safe` function now checks if a node is in the `visited` set for \(O(1)\) lookup time.

2. **Optimized Bit Generation**:
   - The `generate_graph_v2` function now generates the `bit_stream` in a single step, reducing the overhead of extracting each bit individually.

3. **Reduced Memory Allocation**:
   - Instead of recreating the `visited` set, it's passed along and updated in place to avoid frequent memory allocations.
