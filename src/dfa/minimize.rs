use std::collections::{HashSet, HashMap};

use crate::dfa::Alphabet;

type StateId = u128;

/// 计算不可区分状态组。
pub fn compute_indistin_state_groups(dfa: &impl super::CompletedDfa) -> IndistinGroups {
    let mut distin_table = PairTable::new(dfa.number_of_states() as usize);
    // 先标记接受状态和非接受状态为可区分状态。
    for state1 in dfa.accept_states() {
        for state2 in 0..dfa.number_of_states() {
            if dfa.accept_states().contains(&state2) {
                continue;
            }
            distin_table.get(*state1, state2).distinguishable = true;
        }
    }

    for state1 in 0..dfa.number_of_states() - 1 {
        for state2 in state1 + 1..dfa.number_of_states() {
            // 如果这两个状态已经被标记为可区分状态，就不用再检查了。
            if distin_table.is_distinguishable(state1, state2) {
                continue;
            }
            // 如果这两个状态不可区分，就检查它们的转移是否可区分。
            let mut temp_relation = Vec::new();
            for input in dfa.alphabet().to_iter() {
                let to1 = dfa.delta(state1, input);
                let to2 = dfa.delta(state2, input);
                if to1 == to2 {
                    continue;
                }
                if distin_table.is_distinguishable(to1, to2) {
                    distin_table.distinguish(state1, state2);
                    break;
                } else {
                    temp_relation.push((to1, to2));
                }
            }
            // 如果这两个状态的转移都不可区分，就将它们的关联关系加入到状态对关联表中。
            if !distin_table.is_distinguishable(state1, state2) {
                for (to1, to2) in temp_relation {
                    distin_table.get(to1, to2).add_relation(state1, state2);
                }
            }
        }
    }
    
    let mut groups = IndistinGroups { groups: Vec::new() };
    distin_table.for_each(|state1, state2, pair| {
        if !pair.distinguishable {
            groups.insert_pair(state1, state2);
        }
    });
    groups
}

fn order_pair(state1: StateId, state2: StateId) -> (StateId, StateId) {
    if state1 < state2 {
        (state1, state2)
    } else {
        (state2, state1)
    }
}

#[derive(Debug)]
pub struct IndistinGroups {
    groups: Vec<HashSet<StateId>>,
}

impl IndistinGroups {

    fn insert_pair(&mut self, state1: StateId, state2: StateId) {
        let (state1, state2) = order_pair(state1, state2);
        let group_index1 = self.groups.iter().position(|group| group.contains(&state1));
        let group_index2 = self.groups.iter().position(|group| group.contains(&state2));

        if let Some(index1) = group_index1 {
            if let Some(index2) = group_index2 {
                if index1 == index2 {
                    return;
                } else {
                    let mut group1 = self.groups.remove(index1);
                    let group2 = self.groups.remove(index2);
                    group1.extend(group2);
                    self.groups.push(group1);
                    // self.groups.last_mut().unwrap()
                }
            } else {
                self.groups[index1].insert(state2);
                // &mut self.groups[index1]
            }
        } else {
            if let Some(index2) = group_index2 {
                self.groups[index2].insert(state1);
                // &mut self.groups[index2]
            } else {
                let mut new_group = HashSet::new();
                new_group.insert(state1);
                new_group.insert(state2);
                self.groups.push(new_group);
                // self.groups.last_mut().unwrap()
            }
        }
    }

    pub fn num_of_groups(&self) -> usize {
        self.groups.len()
    }

    pub fn num_of_indistin_states(&self) -> usize {
        self.groups.iter().map(|group| group.len()).sum()
    }

    pub fn contains_at(&self, state: StateId) -> Option<usize> {
        self.groups.iter().position(|group| group.contains(&state))
    }

    pub fn iter(&self) -> impl Iterator<Item = &HashSet<StateId>> {
        self.groups.iter()
    }

    /// 假设↓指向一组不可区分状态，⇓指向另一组不可区分状态，x代表一个状态，
    /// 
    /// 在remap之前，状态列表的分布是：
    /// ```
    ///  ↓    ↓ ↓         ⇓  ⇓
    /// xxxxxxxxxxxxxxxxxxxxxxxx
    /// ```
    /// 在remap之后，状态列表的分布是：
    /// ```
    ///                    ↓⇓
    /// xxxxxxxxxxxxxxxxxxxxx
    /// ```
    /// 返回值是remap前后的状态id的映射。
    pub fn remap(&self, max_len: StateId) -> HashMap<StateId, StateId> {
        let mut id_map = HashMap::new();
        let mut new_id: StateId = 0;
        let number_of_distin = max_len - self.num_of_indistin_states() as StateId;

        for old_id in 0..max_len {
            if let Some(group_id) = self.contains_at(old_id) {
                id_map.insert(old_id, number_of_distin + group_id as StateId);
            } else {
                id_map.insert(old_id, new_id);
                new_id += 1;
            }
        }

        // 执行完成后，new_id应该等于可区分状态数量。
        assert_eq!(new_id, number_of_distin);
        
        id_map
    }
}

/// 二维数组实现的可区分状态表，包括状态对关联表。
struct PairTable {
    table: Vec<Vec<StatePair>>,
}

impl PairTable {
    fn new(state_num: usize) -> Self {
        let mut table = Vec::new();
        for _ in 0..state_num - 1 {
            let mut row = Vec::new();
            for _ in 0..state_num {
                row.push(StatePair::new());
            }
            table.push(row);
        }
        Self { table }
    }
    fn get(&mut self, state1: StateId, state2: StateId) -> &mut StatePair {
        let (state1, state2) = order_pair(state1, state2);
        &mut self.table[state1 as usize][state2 as usize]
    }
    fn is_distinguishable(&self, state1: StateId, state2: StateId) -> bool {
        let (state1, state2) = order_pair(state1, state2);
        self.table[state1 as usize][state2 as usize].distinguishable
    }
    fn distinguish(&mut self, state1: StateId, state2: StateId) {
        let (state1, state2) = order_pair(state1, state2);
        self.table[state1 as usize][state2 as usize].distinguishable = true;

        if self.get(state1, state2).associated.is_empty() {
            return;
        }

        let associated_pairs = std::mem::replace(&mut self.get(state1, state2).associated, vec![]);

        for (s1, s2) in associated_pairs.into_iter() {
            self.distinguish(s1, s2);
            // 注意，这是一个递归，需要特别小心检查是否会无限递归。
        }
    }

    fn for_each(&self, mut f: impl FnMut(StateId, StateId, &StatePair)) {
        for state1 in 0..self.table.len() - 1 {
            for state2 in state1 + 1..self.table.len() {
                f(
                    state1 as StateId,
                    state2 as StateId,
                    &self.table[state1][state2],
                );
            }
        }
    }
}
struct StatePair {
    /// 状态对关联表，大部分情况是空表，但是rust对空Vec的内存占用是0，因此不用担心内存占用。
    associated: Vec<(StateId, StateId)>,
    distinguishable: bool,
}

impl StatePair {
    fn new() -> Self {
        Self {
            associated: Vec::new(),
            distinguishable: false,
        }
    }
    fn add_relation(&mut self, state1: StateId, state2: StateId) {
        let (state1, state2) = order_pair(state1, state2);
        self.associated.push((state1, state2));
    }
}
