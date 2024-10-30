use primitive_types::U256;
use rand_mt::Mt19937GenRand64;
use std::collections::HashSet;
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
