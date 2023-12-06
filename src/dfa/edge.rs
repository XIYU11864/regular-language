use super::CompletedDfa;

type StateId = u128;

/// 以边集的形式储存的DFA。
struct DfaAsEdges {
    trans: Vec<Edge>,
}
impl DfaAsEdges {
    fn new_from_dense(dense_dfa: &super::DenseDFA) -> Self {
        let mut trans = Vec::new();
        for from in 1..dense_dfa.number_of_states() {
            for input_index in 0..dense_dfa.alphabet().len() {
                let input = dense_dfa.alphabet[input_index];
                let to = dense_dfa.delta(from, input);
                if to == 0 {
                    continue;
                }
                trans.push(Edge(from, input, to));
            }
        }
        DfaAsEdges { trans }
    }

    fn search_trans_by(
        &self,
        from: StateId,
        to: StateId,
        f: impl Fn(&Edge, (StateId, StateId)) -> bool,
    ) -> Vec<usize> {
        self.trans
            .iter()
            .enumerate()
            .filter(|(_, e)| f(*e, (from, to)))
            .map(|(i, _)| i)
            .collect()
    }

    fn search_trans_by_from(&self, from: StateId) -> Vec<usize> {
        self.search_trans_by(from, 0, |edge, (id, _)| edge.from() == id)
    }

    fn search_trans_by_to(&self, to: StateId) -> Vec<usize> {
        self.search_trans_by(0, to, |edge, (_, id)| edge.to() == id)
    }

    fn search_trans_by_both(&self, from: StateId, to: StateId) -> Vec<usize> {
        self.search_trans_by(from, to, |edge, (f, t)| edge.from() == f && edge.to() == t)
    }
}
struct Edge(StateId, u8, StateId);

impl Edge {
    fn from(&self) -> StateId {
        self.0
    }
    fn to(&self) -> StateId {
        self.2
    }
    fn input(&self) -> u8 {
        self.1
    }
}
