#![deny(unused_must_use)]
//! Integration tests of the Subset protocol.

extern crate env_logger;
extern crate hbbft;
#[macro_use]
extern crate log;
extern crate pairing;
extern crate rand;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rand_derive;
extern crate threshold_crypto as crypto;

mod network;

use std::collections::{BTreeMap, BTreeSet};
use std::iter::once;
use std::sync::Arc;

use hbbft::messaging::NetworkInfo;
use hbbft::subset::Subset;

use network::{Adversary, MessageScheduler, NodeUid, SilentAdversary, TestNetwork, TestNode};

type ProposedValue = Vec<u8>;

fn test_subset<A: Adversary<Subset<NodeUid>>>(
    mut network: TestNetwork<A, Subset<NodeUid>>,
    inputs: &BTreeMap<NodeUid, ProposedValue>,
) {
    let ids: Vec<NodeUid> = network.nodes.keys().cloned().collect();

    for id in ids {
        if let Some(value) = inputs.get(&id) {
            network.input(id, value.to_owned());
        }
    }

    // Terminate when all good nodes do.
    while !network.nodes.values().all(TestNode::terminated) {
        network.step();
    }
    // Verify that all instances output the same set.
    let mut expected = None;
    for node in network.nodes.values() {
        if let Some(output) = expected.as_ref() {
            assert!(once(output).eq(node.outputs()));
            continue;
        }
        assert_eq!(1, node.outputs().len());
        expected = Some(node.outputs()[0].clone());
    }
    let output = expected.unwrap();
    assert!(once(&output).eq(network.observer.outputs()));
    // The Subset algorithm guarantees that more than two thirds of the proposed elements
    // are in the set.
    assert!(output.len() * 3 > inputs.len() * 2);
    // Verify that the set's elements match the proposed values.
    for (id, value) in output {
        assert_eq!(inputs[&id], value);
    }
}

fn new_network<A, F>(
    good_num: usize,
    bad_num: usize,
    adversary: F,
) -> TestNetwork<A, Subset<NodeUid>>
where
    A: Adversary<Subset<NodeUid>>,
    F: Fn(BTreeMap<NodeUid, Arc<NetworkInfo<NodeUid>>>) -> A,
{
    // This returns an error in all but the first test.
    let _ = env_logger::try_init();

    let new_subset =
        |netinfo: Arc<NetworkInfo<NodeUid>>| Subset::new(netinfo, 0).expect("new Subset instance");
    TestNetwork::new(good_num, bad_num, adversary, new_subset)
}

#[test]
fn test_subset_3_out_of_4_nodes_propose() {
    let proposed_value = Vec::from("Fake news");
    let proposing_ids: BTreeSet<NodeUid> = (0..3).map(NodeUid).collect();
    let proposals: BTreeMap<NodeUid, ProposedValue> = proposing_ids
        .iter()
        .map(|id| (*id, proposed_value.clone()))
        .collect();
    let adversary = |_| SilentAdversary::new(MessageScheduler::First);
    let network = new_network(3, 1, adversary);
    test_subset(network, &proposals);
}

#[test]
fn test_subset_5_nodes_different_proposed_values() {
    let proposed_values = vec![
        Vec::from("Alpha"),
        Vec::from("Bravo"),
        Vec::from("Charlie"),
        Vec::from("Delta"),
        Vec::from("Echo"),
    ];
    let proposals: BTreeMap<NodeUid, ProposedValue> = (0..5)
        .into_iter()
        .map(NodeUid)
        .zip(proposed_values)
        .collect();
    let adversary = |_| SilentAdversary::new(MessageScheduler::Random);
    let network = new_network(5, 0, adversary);
    test_subset(network, &proposals);
}

#[test]
fn test_subset_1_node() {
    let proposals: BTreeMap<NodeUid, ProposedValue> =
        once((NodeUid(0), Vec::from("Node 0 is the greatest!"))).collect();
    let adversary = |_| SilentAdversary::new(MessageScheduler::Random);
    let network = new_network(1, 0, adversary);
    test_subset(network, &proposals);
}