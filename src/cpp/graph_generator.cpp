#include "graph_generator.h"

GraphGenerator::GraphGenerator(uint64_t seed, uint64_t range) 
    : prng(seed), distribution(0, range - 1) {
}

uint64_t GraphGenerator::next_random() {
    return distribution(prng);
}

std::unique_ptr<GraphGenerator> create_graph_generator(uint64_t seed, uint64_t range) {
    return std::make_unique<GraphGenerator>(seed, range);
}
