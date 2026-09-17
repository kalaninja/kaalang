use std::collections::BTreeSet;

use super::close;

#[test]
fn closure_matches_a_walk_for_every_five_vertex_dag_in_each_order() {
    for order in [[0, 1, 2, 3, 4], [4, 3, 2, 1, 0], [2, 4, 0, 3, 1]] {
        let edges = (0..order.len())
            .flat_map(|to| (0..to).map(move |from| (order[to], order[from])))
            .collect::<Vec<_>>();
        for mask in 0..1 << edges.len() {
            let mut relation = vec![BTreeSet::new(); order.len()];
            for (bit, &(to, from)) in edges.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    relation[to].insert(from);
                }
            }
            let expected = relation
                .iter()
                .map(|direct| {
                    let mut reached = BTreeSet::new();
                    let mut pending = direct.iter().copied().collect::<Vec<_>>();
                    while let Some(related) = pending.pop() {
                        if reached.insert(related) {
                            pending.extend(&relation[related]);
                        }
                    }
                    reached
                })
                .collect::<Vec<_>>();
            close(&mut relation, order.into_iter());
            assert_eq!(relation, expected, "order {order:?}, edges {mask:b}");
        }
    }
}

#[test]
fn closure_handles_empty_relations_and_chains_across_word_boundaries() {
    for count in [0, 1, 63, 64, 65, 129] {
        let mut preceding = vec![BTreeSet::new(); count];
        let mut following = preceding.clone();
        for index in 1..count {
            preceding[index].insert(index - 1);
            following[index - 1].insert(index);
        }
        close(&mut preceding, 0..count);
        close(&mut following, (0..count).rev());
        for index in 0..count {
            assert_eq!(preceding[index], (0..index).collect());
            assert_eq!(following[index], (index + 1..count).collect());
        }
    }
}
