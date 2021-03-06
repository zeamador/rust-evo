// Inspired by NEAT: "Evolving Neural Networks through Augmenting Topologies"
// by Kenneth O. Stanley and Risto Miikkulainen
// http://nn.cs.utexas.edu/downloads/papers/stanley.ec02.pdf

use rand::{Rng, SeedableRng};
use rand_distr::StandardNormal;
use rand_pcg::Pcg64Mcg;
use std::f32;
use std::fmt;
use std::fmt::{Error, Formatter};

type Coefficient = f32;
type VecIndex = u16;
type NodeValue = f32;

#[derive(Clone, Debug, PartialEq)]
pub struct SparseNeuralNet {
    genome: SparseNeuralNetGenome,
    node_values: Vec<NodeValue>,
}

impl SparseNeuralNet {
    pub fn new(genome: SparseNeuralNetGenome) -> Self {
        let num_nodes = genome.num_nodes;
        SparseNeuralNet {
            genome,
            node_values: vec![0.0; num_nodes as usize],
        }
    }

    pub fn spawn(&self, randomness: &mut dyn MutationRandomness) -> Self {
        Self::new(self.genome.spawn(randomness))
    }

    pub fn set_node_value(&mut self, index: VecIndex, value: NodeValue) {
        self.node_values[index as usize] = value;
    }

    pub fn node_value(&self, index: VecIndex) -> NodeValue {
        self.node_values[index as usize]
    }

    pub fn run(&mut self) {
        self.genome.run(&mut self.node_values);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SparseNeuralNetGenome {
    ops: Vec<Op>,
    transfer_fn: TransferFn,
    num_nodes: VecIndex,
}

impl SparseNeuralNetGenome {
    pub fn new(transfer_fn: TransferFn) -> Self {
        SparseNeuralNetGenome {
            ops: vec![],
            transfer_fn,
            num_nodes: 0,
        }
    }

    pub fn connect_node(
        &mut self,
        to_value_index: VecIndex,
        bias: Coefficient,
        from_value_weights: &[(VecIndex, Coefficient)],
    ) {
        self.grow_num_nodes_if_needed(to_value_index);
        self.ops.push(Op::Bias {
            value_index: to_value_index,
            bias,
        });
        for (from_value_index, weight) in from_value_weights {
            self.grow_num_nodes_if_needed(*from_value_index);
            self.ops.push(Op::Connection {
                from_value_index: *from_value_index,
                to_value_index,
                weight: *weight,
            });
        }
        self.ops.push(Op::Transfer {
            value_index: to_value_index,
            transfer_fn: self.transfer_fn,
        });
    }

    fn grow_num_nodes_if_needed(&mut self, new_index: VecIndex) {
        self.num_nodes = self.num_nodes.max(new_index + 1);
    }

    fn run(&self, node_values: &mut [NodeValue]) {
        for op in &self.ops {
            op.run(node_values);
        }
    }

    pub fn spawn(&self, randomness: &mut dyn MutationRandomness) -> Self {
        SparseNeuralNetGenome {
            ops: Self::copy_with_mutated_weights(&self.ops, randomness),
            transfer_fn: self.transfer_fn,
            num_nodes: self.num_nodes,
        }
    }

    fn copy_with_mutated_weights(ops: &[Op], randomness: &mut dyn MutationRandomness) -> Vec<Op> {
        ops.iter()
            .map(|op| op.copy_with_mutated_weight(|weight| randomness.mutate_weight(weight)))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Bias {
        value_index: VecIndex,
        bias: Coefficient,
    },
    Connection {
        from_value_index: VecIndex,
        to_value_index: VecIndex,
        weight: Coefficient,
    },
    Transfer {
        value_index: VecIndex,
        transfer_fn: TransferFn,
    },
}

impl Op {
    fn run(&self, node_values: &mut [NodeValue]) {
        match self {
            Self::Bias { value_index, bias } => {
                let value = &mut node_values[*value_index as usize];
                *value = *bias;
            }

            Self::Connection {
                from_value_index,
                to_value_index,
                weight,
            } => {
                let from_value = node_values[*from_value_index as usize];
                let to_value = &mut node_values[*to_value_index as usize];
                *to_value += *weight * from_value;
            }

            Self::Transfer {
                value_index,
                transfer_fn,
            } => {
                let value = &mut node_values[*value_index as usize];
                transfer_fn.call(value);
            }
        }
    }

    fn copy_with_mutated_weight<F>(&self, mut mutate_weight: F) -> Self
    where
        F: FnMut(Coefficient) -> Coefficient,
    {
        match self {
            Self::Bias { value_index, bias } => Self::Bias {
                value_index: *value_index,
                bias: mutate_weight(*bias),
            },

            Self::Connection {
                from_value_index,
                to_value_index,
                weight,
            } => Self::Connection {
                from_value_index: *from_value_index,
                to_value_index: *to_value_index,
                weight: mutate_weight(*weight),
            },

            Self::Transfer {
                value_index,
                transfer_fn,
            } => Self::Transfer {
                value_index: *value_index,
                transfer_fn: *transfer_fn,
            },
        }
    }
}

#[derive(Copy)]
pub struct TransferFn {
    the_fn: fn(&mut NodeValue),
}

impl TransferFn {
    pub const IDENTITY: TransferFn = TransferFn {
        the_fn: Self::identity,
    };
    pub const SIGMOIDAL: TransferFn = TransferFn {
        the_fn: Self::sigmoidal,
    };

    pub fn new(the_fn: fn(&mut NodeValue)) -> Self {
        TransferFn { the_fn }
    }

    pub fn call(self, value: &mut NodeValue) {
        (self.the_fn)(value)
    }

    fn identity(_value: &mut NodeValue) {}

    fn sigmoidal(value: &mut NodeValue) {
        *value = Self::sigmoidal_fn(*value);
    }

    fn sigmoidal_fn(val: NodeValue) -> NodeValue {
        1.0_f32 / (1.0_f32 + (-4.9_f32 * val).exp())
    }
}

impl Clone for TransferFn {
    fn clone(&self) -> Self {
        *self
    }
}

impl fmt::Debug for TransferFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        // TODO match against constants and print name?
        write!(f, "{}", self.the_fn as usize)
    }
}

impl PartialEq for TransferFn {
    fn eq(&self, other: &Self) -> bool {
        self.the_fn as usize == other.the_fn as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MutationParameters {
    pub weight_mutation_probability: f32,
    pub weight_mutation_stdev: f32,
    pub add_node_probability: f32,
}

impl MutationParameters {
    pub const NO_MUTATION: MutationParameters = MutationParameters {
        weight_mutation_probability: 0.0,
        weight_mutation_stdev: 0.0,
        add_node_probability: 0.0,
    };

    fn _validate(&self) {
        assert!(Self::_is_probability(self.weight_mutation_probability));
    }

    fn _is_probability(num: f32) -> bool {
        0.0 <= num && num <= 1.0
    }
}

pub trait MutationRandomness {
    fn mutate_weight(&mut self, weight: Coefficient) -> Coefficient;
}

#[derive(Clone, Debug)]
pub struct SeededMutationRandomness {
    rng: Pcg64Mcg,
    mutation_parameters: &'static MutationParameters,
}

impl SeededMutationRandomness {
    pub fn new(seed: u64, mutation_parameters: &'static MutationParameters) -> Self {
        SeededMutationRandomness {
            rng: rand_pcg::Pcg64Mcg::seed_from_u64(seed),
            mutation_parameters,
        }
    }

    pub fn spawn(&mut self) -> Self {
        Self::new(self.child_seed(), self.mutation_parameters)
    }

    pub fn child_seed(&mut self) -> u64 {
        self.rng.gen()
    }

    fn should_mutate_this_weight(&mut self) -> bool {
        self.rng
            .gen_bool(self.mutation_parameters.weight_mutation_probability as f64)
    }
}

impl MutationRandomness for SeededMutationRandomness {
    fn mutate_weight(&mut self, weight: Coefficient) -> Coefficient {
        if !self.should_mutate_this_weight() {
            return weight;
        }

        let gaussian = self.rng.sample::<f32, _>(StandardNormal);
        weight + gaussian * self.mutation_parameters.weight_mutation_stdev * weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_layer_sparsely_connected() {
        let mut genome = SparseNeuralNetGenome::new(TransferFn::new(plus_one));
        genome.connect_node(2, 0.5, &[(0, 0.5)]);
        genome.connect_node(3, 0.0, &[(0, 0.75), (1, 0.25)]);

        let mut nnet = SparseNeuralNet::new(genome);
        nnet.set_node_value(0, 2.0);
        nnet.set_node_value(1, 4.0);
        nnet.run();

        assert_eq!(nnet.node_value(2), 2.5);
        assert_eq!(nnet.node_value(3), 3.5);
    }

    #[test]
    fn run_clears_previous_values() {
        let mut genome = SparseNeuralNetGenome::new(TransferFn::IDENTITY);
        genome.connect_node(1, 0.0, &[(0, 1.0)]);

        let mut nnet = SparseNeuralNet::new(genome);
        nnet.set_node_value(0, 1.0);
        nnet.run();
        nnet.set_node_value(0, 3.0);
        nnet.run();

        assert_eq!(nnet.node_value(1), 3.0);
    }

    #[test]
    fn three_layer() {
        let mut genome = SparseNeuralNetGenome::new(TransferFn::IDENTITY);
        genome.connect_node(1, 0.5, &[(0, 0.5)]);
        genome.connect_node(2, 0.0, &[(1, 0.5)]);

        let mut nnet = SparseNeuralNet::new(genome);
        nnet.set_node_value(0, 2.0);
        nnet.run();

        assert_eq!(nnet.node_value(2), 0.75);
    }

    #[test]
    fn recurrent_connection() {
        let mut genome = SparseNeuralNetGenome::new(TransferFn::IDENTITY);
        genome.connect_node(1, 0.0, &[(0, 1.0), (2, 2.0)]);
        genome.connect_node(2, 0.0, &[(1, 1.0)]);

        let mut nnet = SparseNeuralNet::new(genome);
        nnet.set_node_value(0, 1.0);
        nnet.run();

        assert_eq!(nnet.node_value(0), 1.0);
        assert_eq!(nnet.node_value(1), 1.0);
        assert_eq!(nnet.node_value(2), 1.0);

        nnet.set_node_value(0, 0.0);
        nnet.run();

        assert_eq!(nnet.node_value(0), 0.0);
        assert_eq!(nnet.node_value(1), 2.0);
        assert_eq!(nnet.node_value(2), 2.0);
    }

    #[test]
    fn spawn_unmutated() {
        let mut genome = SparseNeuralNetGenome::new(TransferFn::SIGMOIDAL);
        genome.connect_node(1, 0.0, &[(0, 1.0), (2, 2.0)]);
        genome.connect_node(2, 0.0, &[(1, 1.0)]);

        let mut randomness = StubMutationRandomness {
            mutated_weights: vec![],
        };
        let copy = genome.spawn(&mut randomness);

        assert_eq!(copy.ops, genome.ops);
        assert_eq!(copy.transfer_fn, TransferFn::SIGMOIDAL);
    }

    #[test]
    fn spawn_with_mutated_weights() {
        let mut genome = SparseNeuralNetGenome::new(TransferFn::SIGMOIDAL);
        genome.connect_node(2, 1.5, &[(0, 1.0), (1, 2.0)]);

        let mut randomness = StubMutationRandomness {
            mutated_weights: vec![(1.5, -0.5), (2.0, 2.25)],
        };
        let copy = genome.spawn(&mut randomness);

        assert_eq!(
            copy.ops,
            vec![
                Op::Bias {
                    value_index: 2,
                    bias: -0.5,
                },
                Op::Connection {
                    from_value_index: 0,
                    to_value_index: 2,
                    weight: 1.0,
                },
                Op::Connection {
                    from_value_index: 1,
                    to_value_index: 2,
                    weight: 2.25,
                },
                Op::Transfer {
                    value_index: 2,
                    transfer_fn: TransferFn::SIGMOIDAL,
                }
            ]
        );
    }

    #[test]
    fn seeded_mutation_randomness_leaves_weight_unmutated() {
        let mut randomness = SeededMutationRandomness::new(0, &MutationParameters::NO_MUTATION);
        assert_eq!(randomness.mutate_weight(1.0), 1.0);
    }

    #[test]
    fn seeded_mutation_randomness_mutates_weight() {
        const ALWAYS_MUTATE: MutationParameters = MutationParameters {
            weight_mutation_probability: 1.0,
            weight_mutation_stdev: 1.0,
            ..MutationParameters::NO_MUTATION
        };

        let mut randomness = SeededMutationRandomness::new(0, &ALWAYS_MUTATE);
        assert_ne!(randomness.mutate_weight(1.0), 1.0);
    }

    fn plus_one(value: &mut NodeValue) {
        *value += 1.0;
    }

    struct StubMutationRandomness {
        mutated_weights: Vec<(Coefficient, Coefficient)>,
    }

    impl MutationRandomness for StubMutationRandomness {
        fn mutate_weight(&mut self, weight: Coefficient) -> Coefficient {
            for (from_weight, to_weight) in &self.mutated_weights {
                if *from_weight == weight {
                    return *to_weight;
                }
            }
            weight
        }
    }
}
