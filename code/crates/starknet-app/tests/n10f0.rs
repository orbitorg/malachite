use std::array;

use malachite_node::config::App;
use malachite_starknet_app::spawn::SpawnStarknetNode;
use malachite_test::utils::test::{Expected, Test, TestNode};

#[tokio::test(flavor = "multi_thread")]
pub async fn all_correct_nodes() {
    let nodes = array::from_fn(|_| TestNode::correct(10));
    let test = Test::<10>::new(nodes, Expected::Exactly(30));

    test.run::<SpawnStarknetNode>(App::Starknet).await
}
