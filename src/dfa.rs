use crate::nfa::NFA;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fmt;

mod edge;

/// DFA的极小化相关的方法。
pub mod minimize;

/// 传入一个集合的索引的子集，返回一个无符号数来*表示*这个子集。
/// 从NFA构造DFA的过程特别需要这个宏。
///
/// 例如，我有一个Vec，里面有8个元素，我想要表示包含这个Vec的第0、1、3、5个元素的子集，
/// 那么我可以传入一个\[0, 1, 3, 5\]的迭代器，返回值为二进制数 00101011。
///
/// 用宏而不用函数的原因是，宏只要写一遍就能适用于所有无符号整数类型，比如u8、u16、u32等。
/// 而用函数实现需要用复杂的泛型来表示传入的数是一个无符号数。
///
/// 但是用宏就没有传入参数的类型检查了。
/// 需要在调用的时候自己保证传入的参数是一个内含无符号数的迭代器。
macro_rules! encode_subset {
    ($subset:expr) => {{
        let mut result = 0;
        for i in $subset {
            // 将 result 的第 i 位设置为 1
            result |= 1 << i;
        }
        result
    }};
}

type StateId = u128;

/// 稀疏DFA的抽象。
///
/// 所谓稀疏，指的是储存状态转移函数的方法。
/// 稀疏DFA定义一个State结构体代表这个DFA中的状态，并把从这个状态出发的状态转移函数储存在State结构体中。
/// 在DFA中，则用HashMap储存所有的状态。
///
/// 与之相对的“稠密”DFA，是指用一个数组储存所有的状态转移函数，而不抽象出State结构体。
trait SparseDFA {
    type State: State;
    type Error;

    fn init_empty() -> Self;
    fn add_empty_state(&mut self, id: StateId) -> &mut Self::State;
    fn add_transition(&mut self, from: StateId, input: u8, to: StateId);
    fn get_state_by_id(&mut self, id: StateId) -> &mut Self::State;
    fn set_start_state(&mut self, id: StateId);
    fn set_accept_state(&mut self, id: StateId);
}

/// 已经构造完成的DFA，可以读取状态转移函数、字母表、开始状态等信息。
pub trait CompletedDfa {
    type Alphabet: Alphabet;

    fn alphabet(&self) -> &Self::Alphabet;
    fn start_state(&self) -> StateId;
    fn accept_states(&self) -> &HashSet<StateId>;
    fn number_of_states(&self) -> StateId;

    /// 将这个DFA转换为Graphviz的dot语言，用于绘制状态转移图。
    fn to_dot(&self) -> String;

    /// delta 是状态转移函数δ的读音。这个函数等价于 δ(from, input)。
    /// 也就是说，这个函数会返回从状态from经过输入input到达的状态。
    fn delta(&self, from: StateId, input: u8) -> StateId;

    fn to_fmt_output(&self) -> String {
        let mut output = String::from("\t0\t1\n");
        let start_state = self.start_state();
        let accept_states = self.accept_states();

        for i in 1..self.number_of_states() {
            if accept_states.contains(&i) {
                output.push('*');
            }
            if i == start_state {
                output.push_str(&format!("#q{}\t", i));
            } else {
                output.push_str(&format!("q{}\t", i));
            }

            macro_rules! state_or_none {
                ($state:expr) => {
                    if $state == 0 {
                        "N".to_string()
                    } else {
                        format!("q{}", $state)
                    }
                };
            }
            let state0_str = state_or_none!(self.delta(i, b'0'));
            let state1_str = state_or_none!(self.delta(i, b'1'));

            output.push_str(&format!("{}\t{}\t", state0_str, state1_str));

            output.push('\n');
        }
        output
    }
}

/// DFA的字母表，可以获取大小，可以转换为迭代器。
pub trait Alphabet {
    type Iter: Iterator<Item = u8>;
    fn len(&self) -> usize;
    fn to_iter(&self) -> Self::Iter;
}

impl Alphabet for (u8, u8) {
    type Iter = std::ops::RangeInclusive<u8>;
    fn len(&self) -> usize {
        2
    }
    fn to_iter(&self) -> Self::Iter {
        (self.0..=self.1).into_iter()
    }
}

impl Alphabet for Vec<u8> {
    type Iter = std::vec::IntoIter<u8>;
    fn len(&self) -> usize {
        self.len()
    }
    fn to_iter(&self) -> Self::Iter {
        self.clone().into_iter()
    }
}

/// 稀疏DFA。
/// 01的意思是这个DFA的字母表只有0和1，适用于大作业给的测试用例。
pub struct DFA01 {
    states: HashMap<StateId, State01>,
    alphabet: (u8, u8),
    start_state: Option<StateId>,
    accept_states: HashSet<StateId>,
}

impl DFA01 {
    /// 获取这个DFA的所有状态的迭代器，并且迭代顺序按照状态编号排序。
    pub fn states_iter(&self) -> impl Iterator<Item = &State01> {
        self.states
            .iter()
            .sorted_by_key(|entry| entry.0)
            .map(|entry| entry.1)
    }

    /// 获取这个DFA的所有状态和其编号的迭代器，并且迭代顺序按照状态编号排序。
    pub fn states_with_id_iter(&self) -> impl Iterator<Item = (&StateId, &State01)> {
        self.states.iter().sorted_by_key(|entry| entry.0)
    }

    /// 将状态转移表转化为DOT格式的状态转移图。
    pub fn call_to_dot(&self) -> String {
        self.to_dot()
    }

    fn search_unreachable_states(&mut self) -> HashSet<StateId> {
        let mut reachable_states = HashSet::new();
        let mut stack = Vec::new();

        if let Some(start_state) = self.start_state {
            stack.push(start_state);
        }

        while let Some(state_id) = stack.pop() {
            reachable_states.insert(state_id);
            let state = self.get_state_by_id(state_id);
            if !reachable_states.contains(&state.zero_to) {
                stack.push(state.zero_to);
            }
            if !reachable_states.contains(&state.one_to) {
                stack.push(state.one_to);
            }
        }

        let all_states: HashSet<_> = self.states.keys().cloned().collect();
        all_states.difference(&reachable_states).cloned().collect()
    }
}

impl DFA01 {
    /// 从NFA构造DFA。
    pub fn build_dfa_from_nfa(nfa: &NFA) -> Self {
        let nfa_state_set_len = nfa.get_states_iter().len();
        if nfa_state_set_len > 128 {
            panic!("too many states");
        }

        let alphabet = nfa.alphabet();
        if alphabet.len() == 2 && alphabet.contains(&b'0') && alphabet.contains(&b'1') {
            // 检查这个NFA的字母表是否只有0和1。
        } else {
            panic!("alphabet is not {{'0','1'}}");
        }

        trait ToDfaStateID {
            /// 将NFA状态ID转换为DFA的状态ID。
            fn to_dfa_state_id(&self) -> StateId;
        }

        macro_rules! impl_to_dfa_state_id {
            ($($t:ty),*) => {
                $(
                    impl ToDfaStateID for $t {
                        fn to_dfa_state_id(&self) -> StateId {
                            1 << *self
                        }
                    }
                )*
            };
        }

        impl_to_dfa_state_id!(u32, usize, u8);

        let mut dfa = Self::init_empty();
        let mut stack = Vec::new();

        dfa.set_start_state(nfa.start_state.unwrap().to_dfa_state_id());

        // 准备好一个HashSet，用来判断一个DFA状态是否直接来自NFA，也就是只包含单个NFA状态的DFA状态。
        // 例如，如果原NFA的状态集合是{0,1,2}，那么DFA中的状态[0]、[1]、[2]都是直接来自NFA的。
        let states_directly_from_nfa: HashSet<_> = (0..nfa_state_set_len)
            .map(|id| id.to_dfa_state_id())
            .collect();

        // 将包含单个NFA状态的DFA状态加入到DFA中。
        for id in 0..nfa_state_set_len {
            // 这里使用add_empty_state方法是因为知道插入的状态一定是新的，不会覆盖掉原状态。
            let new_state = dfa.add_empty_state(id.to_dfa_state_id());
            for (input, targets) in nfa.deltas(id as u32) {
                let to = encode_subset!(targets.into_iter());
                new_state.add_transition(input, to);

                if !states_directly_from_nfa.contains(&to) {
                    stack.push(to);
                }
            }
        }

        while let Some(state_id) = stack.pop() {
            let mut subset = Vec::new();

            // 实际上，一个DFA状态的id就是一个NFA状态的集合的编码。
            let mut encoded_subset = state_id;

            // 这里用u8的原因是因为bit表示的是位数，u128有128位，
            // u8能表示0~255，已经足够了一倍。
            let mut bit: u8 = 0;

            while encoded_subset != 0 {
                if encoded_subset & 1 == 1 {
                    subset.push(bit.to_dfa_state_id());
                }
                bit += 1;
                encoded_subset >>= 1;
            }
            // 这里的subset相当于把state_id的每一位拆开了。
            // 比如，假设state_id = 11010,（二进制表示）
            // 那么subset就包括：
            // [10000,
            //  01000,
            //  00010]
            // 拆开的每一个数都代表一个DFA状态的id。

            let (zero_to, one_to) = subset
                .iter()
                .map(|id| {
                    let state = dfa.get_state_by_id(*id);
                    (state.zero_to, state.one_to)
                })
                .reduce(|(zero_to1, one_to1), (zero_to2, one_to2)| {
                    (zero_to1 | zero_to2, one_to1 | one_to2)
                })
                .unwrap_or((0, 0));
            // 上面的|是按位或。
            // 因为DFA的状态id是一个NFA状态的集合的编码，将两个DFA的状态id按位或，就相当于求并集。

            let state = dfa.get_state_by_id(state_id);
            state.one_to = one_to;
            state.zero_to = zero_to;

            if !dfa.states.keys().contains(&one_to) {
                stack.push(one_to);
            }
            if !dfa.states.keys().contains(&zero_to) {
                stack.push(zero_to);
            }
        }

        // 删除不可达状态
        for state_id in dfa.search_unreachable_states() {
            dfa.states.remove(&state_id);
        }
        // 标记接受状态
        for id in dfa.states.keys() {
            // 因为NFA的构造方法，我们知道NFA只有一个接受状态。
            // 如果要将更普遍的NFA转化为DFA，这里需要修改。
            if *id & nfa.accept_states[0].to_dfa_state_id() != 0 {
                dfa.accept_states.insert(*id);
            }
        }
        dfa
    }
}

impl SparseDFA for DFA01 {
    type State = State01;

    type Error = String;

    fn init_empty() -> Self {
        Self {
            states: HashMap::new(),
            alphabet: (b'0', b'1'),
            start_state: None,
            accept_states: HashSet::new(),
        }
    }

    /// 这个方法会根据传入的id插入一个空状态，然后返回这个状态的可变引用。
    /// 如果此id已经存在一个对应的状态，这个方法会覆盖掉原状态，因此不推荐使用此方法，除非保证传入的id一定是新的。
    fn add_empty_state(&mut self, id: StateId) -> &mut Self::State {
        // 先插入到HashMap中，再取出可变引用，这样新状态的所有权属于HashMap，不会被释放。
        self.states.insert(id, State01::new());
        self.states.get_mut(&id).unwrap()
    }

    /// 传入一个状态的id，返回这个状态的可变引用。
    /// 如果这个状态不存在，会先插入一个空状态，再返回这个状态的可变引用。
    fn get_state_by_id(&mut self, id: StateId) -> &mut Self::State {
        self.states.entry(id).or_insert(State01::new())
    }

    fn add_transition(&mut self, from: StateId, input: u8, to: StateId) {
        let from = self.states.get_mut(&from).unwrap();
        from.add_transition(input, to);
    }

    fn set_start_state(&mut self, id: StateId) {
        self.start_state = Some(id);
    }

    fn set_accept_state(&mut self, id: StateId) {
        self.accept_states.insert(id);
    }
}

impl CompletedDfa for DFA01 {
    /// 由于这个DFA的字母表只有0和1，所以直接用一个有两个元素的元组来表示字母表。
    type Alphabet = (u8, u8);
    fn alphabet(&self) -> &Self::Alphabet {
        &self.alphabet
    }

    fn start_state(&self) -> StateId {
        self.start_state.unwrap()
    }

    fn accept_states(&self) -> &HashSet<StateId> {
        &self.accept_states
    }

    fn number_of_states(&self) -> StateId {
        self.states.len() as StateId
    }

    fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph DFA {\n");
        dot.push_str("rankdir=LR;\n");
        dot.push_str("node [shape = doublecircle];\n");
        for state_id in &self.accept_states {
            dot.push_str(&format!("{};\n", state_id));
        }
        dot.push_str("node [shape = circle];\n");
        for (id, state) in self.states_with_id_iter() {
            if state.zero_to != 0 {
                dot.push_str(&format!("{} -> {} [label = \"0\"];\n", id, state.zero_to));
            }
            if state.one_to != 0 {
                dot.push_str(&format!("{} -> {} [label = \"1\"];\n", id, state.one_to));
            }
        }
        dot.push_str("}\n");
        dot
    }

    fn delta(&self, from: StateId, input: u8) -> StateId {
        let state = self.states.get(&from).expect("No such a state");
        match input {
            b'0' => state.zero_to,
            b'1' => state.one_to,
            _ => panic!("invalid input"),
        }
    }
}

trait State {
    type StateId;
    type Transitions;
    fn transitions(&self) -> Self::Transitions;
}

/// 用于表示`DFA01`这个结构体的状态。
pub struct State01 {
    zero_to: StateId,
    one_to: StateId,
}

impl State01 {
    fn new() -> Self {
        Self {
            zero_to: 0,
            one_to: 0,
        }
    }
}

impl State01 {
    fn add_transition(&mut self, input: u8, to: StateId) {
        match input {
            b'0' => self.zero_to = to,
            b'1' => self.one_to = to,
            _ => panic!("invalid input"),
        }
    }
}

impl State for State01 {
    type StateId = StateId;
    type Transitions = (StateId, StateId);

    fn transitions(&self) -> Self::Transitions {
        (self.zero_to, self.one_to)
    }
}

/// 稠密DFA的实现。
///
/// 储存了两份状态转移函数表。
/// 一份 `out_transitions` 以出发状态为索引，称为“出表”；
/// 一份 `in_transitions` 以到达状态为索引，称为“入表”。
///
/// 本来感觉多储存一份入表可以方便之后使用DFA构造正则表达式，但实际上好像没什么帮助。暂时没有删除。
pub struct DenseDFA {
    alphabet: Vec<u8>,
    out_transitions: Transisions<StateId>,
    in_transitions: Transisions<Vec<StateId>>,
    start_state: Option<StateId>,
    accept_states: HashSet<StateId>,
}

impl DenseDFA {
    fn add_transition(&mut self, from: StateId, input: u8, to: StateId) {
        // dbg!(from, to, self.in_transitions.stride());

        let from_index =
            (from as usize) * self.out_transitions.stride() + self.alphabet_index_of(input);

        self.out_transitions.trans[from_index] = to;

        let to_index = (to as usize) * self.in_transitions.stride() + self.alphabet_index_of(input);

        self.in_transitions.trans[to_index].push(from);
    }

    fn set_start_state(&mut self, id: StateId) {
        self.start_state = Some(id);
    }

    fn set_accept_state(&mut self, id: StateId) {
        self.accept_states.insert(id);
    }
}

impl CompletedDfa for DenseDFA {
    /// 使用一个Vec来表示字母表。不用HashSet的原因是需要字母表是有序的。
    type Alphabet = Vec<u8>;

    fn number_of_states(&self) -> StateId {
        self.out_transitions.number_of_states() as StateId
    }

    fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph DFA {\n");
        dot.push_str("rankdir=LR;\n");
        dot.push_str("node [shape = doublecircle];\n");
        for state_id in &self.accept_states {
            dot.push_str(&format!("{};\n", state_id));
        }
        dot.push_str("node [shape = circle];\n");
        let stride2 = self.out_transitions.stride_as_power_of_2;
        for (index, to) in self.out_transitions.trans.iter().enumerate() {
            let from = index >> stride2;
            // 如果想显示陷阱状态，就把下面这个if注释掉。
            if *to == 0 || from == 0 {
                continue;
            }
            let input = self.alphabet[index & ((1 << stride2) - 1)];
            dot.push_str(&format!(
                "{} -> {} [label = \"{}\"];\n",
                from, to, input as char
            ));
        }
        dot.push_str("}\n");
        dot
    }

    /// 输入给定的状态id和输入字符，返回下一个状态的索引。
    fn delta(&self, from: StateId, input: u8) -> StateId {
        if from > self.out_transitions.number_of_states() as StateId {
            panic!("no such a state: {}", from)
        }
        if !self.alphabet.contains(&input) {
            panic!("no such a input: {}", input as char)
        }
        self.out_transitions.trans[(from << self.out_transitions.stride_as_power_of_2) as usize
            + self.alphabet_index_of(input)]
    }

    fn alphabet(&self) -> &Self::Alphabet {
        &self.alphabet
    }

    fn start_state(&self) -> StateId {
        self.start_state.unwrap()
    }

    fn accept_states(&self) -> &HashSet<StateId> {
        &self.accept_states
    }
}

#[derive(Clone)]
struct Transisions<T> {
    trans: Vec<T>,
    // stride: usize,
    stride_as_power_of_2: u8,
}

impl Transisions<StateId> {
    fn new_with_num_and_stride(number_of_states: usize, alghabet_len: usize) -> Self {
        // alghabet_len是一个小于256的数，因此它的二进制表示最多只有8位。
        let stride = alghabet_len.next_power_of_two();
        // dbg!(stride.trailing_zeros());
        Transisions {
            trans: vec![0; number_of_states * stride],
            stride_as_power_of_2: stride.trailing_zeros() as u8,
        }
    }
}

impl<T> Transisions<T> {
    fn stride(&self) -> usize {
        // dbg!(self.stride_as_power_of_2);
        1 << self.stride_as_power_of_2
    }
    fn number_of_states(&self) -> usize {
        self.trans.len() >> self.stride_as_power_of_2
    }
}

impl Transisions<Vec<StateId>> {
    fn new_with_num_and_stride(number_of_states: usize, alghabet_len: usize) -> Self {
        // alghabet_len是一个小于256的数，因此它的二进制表示最多只有8位。
        let stride = alghabet_len.next_power_of_two();
        Transisions {
            trans: vec![Vec::<StateId>::new(); number_of_states * stride],
            stride_as_power_of_2: stride.trailing_zeros() as u8,
        }
    }
}

struct DfaConfig {
    number_of_states: usize,
    alphabet: Vec<u8>,
    start_state_id: StateId,
    accept_states: HashSet<StateId>,

    // 用一个HashMap来记录新的状态id和旧的状态id的对应关系。
    // key是旧的状态id，value是新的状态id。
    id_map: HashMap<StateId, StateId>,
}

impl DfaConfig {
    fn new_from_01(dfa: &DFA01) -> Self {
        DfaConfig {
            number_of_states: dfa.states.len(),
            alphabet: vec![dfa.alphabet.0, dfa.alphabet.1],
            start_state_id: dfa.start_state.unwrap(),
            accept_states: dfa.accept_states.clone(),
            id_map: dfa
                .states_with_id_iter()
                .enumerate()
                .map(|(new_id, (old_id, _))| (*old_id, new_id as StateId))
                .collect(),
        }
    }

    /// 将原来的不可区分状态合并为一个状态，返回一个新的DFA配置。
    /// 具体方法是，有几组不可区分状态，就新添加几个状态。然后把每一组的状态都映射到新的状态上。
    fn new_for_minimize(dfa: &DenseDFA, indistin: &minimize::IndistinGroups) -> Self {
        let id_map = indistin.remap(dfa.number_of_states());
        dbg!(&id_map);
        dbg!(&dfa.accept_states);
        DfaConfig {
            number_of_states: dfa.number_of_states() as usize - indistin.num_of_indistin_states()
                + indistin.num_of_groups(),
            alphabet: dfa.alphabet.clone(),
            start_state_id: dfa.start_state.unwrap(),
            accept_states: dfa.accept_states.clone(),
            id_map,
        }
    }
}

impl DenseDFA {
    fn init_with_config(config: &DfaConfig) -> Self {
        let len = config.alphabet.len();
        DenseDFA {
            alphabet: config.alphabet.clone(),
            out_transitions: Transisions::<StateId>::new_with_num_and_stride(
                config.number_of_states,
                len,
            ),
            in_transitions: Transisions::<Vec<StateId>>::new_with_num_and_stride(
                config.number_of_states,
                len,
            ),
            start_state: Some(config.id_map[&config.start_state_id]),
            accept_states: config
                .accept_states
                .iter()
                .map(|id| config.id_map[&id])
                .collect(),
        }
    }

    /// delta 的意思是状态转移函数。
    fn delta_by_tran_index(&self, index: usize) -> StateId {
        // 如果index超出了范围，会panic。
        self.out_transitions.trans[index]
    }

    fn is_no_way_out(&self, state: StateId) -> bool {
        self.out_transitions.trans[(state << self.out_transitions.stride_as_power_of_2) as usize
            ..((state + 1) << self.out_transitions.stride_as_power_of_2) as usize]
            .iter()
            .all(|&to| to == 0)
    }

    fn alphabet_index_of(&self, input: u8) -> usize {
        self.alphabet
            .to_iter()
            .position(|x| x == input)
            .expect("invalid input")
    }

    fn clear_accept_states(&mut self) {
        self.accept_states.clear();
    }

    /// 从稀疏DFA构造稠密DFA。
    pub fn build_from_sparse01_dfa(sparse_dfa: &DFA01) -> Self {
        let config = DfaConfig::new_from_01(sparse_dfa);
        let mut dense_dfa = Self::init_with_config(&config);

        for (new_id, state) in sparse_dfa.states_iter().enumerate() {
            dense_dfa.add_transition(new_id as StateId, b'0', config.id_map[&state.zero_to]);
            dense_dfa.add_transition(new_id as StateId, b'1', config.id_map[&state.one_to]);
        }
        dense_dfa
    }

    pub fn test_print_in_transitions(&self) {
        let stride2 = self.in_transitions.stride_as_power_of_2;
        for (index, froms) in self.in_transitions.trans.iter().enumerate() {
            let state_id = index >> stride2;
            let input = self.alphabet[index & ((1 << stride2) - 1)];
            for from in froms {
                println!("{} <- {} ({})", state_id, from, input as char);
            }
        }
    }

    /// 将这个DFA转换为正则文法。
    pub fn to_rg(&self) -> String {
        let mut rg = String::new();
        rg.push_str(&format!("S -> q{}\n", self.start_state()));
        for from in 1..self.number_of_states() {
            // 这个变量代表产生式的右部，也就是候选式。
            let mut candidate = String::new();
            for input in self.alphabet.to_iter() {
                let to = self.delta(from, input);
                if self.accept_states.contains(&to) {
                    candidate.push_str(&format!(" {} |", input as char));
                }
                if to == 0 || self.is_no_way_out(to) {
                    continue;
                }
                candidate.push_str(&format!(" {}q{} |", input as char, to));
            }
            if let Some(_) = candidate.pop() {
                rg.push_str(&format!("q{} ->{}\n", from, candidate));
            }
        }
        rg
    }

    /// 将状态转移表转化为DOT语言表示的状态转移图。
    pub fn call_to_dot(&self) -> String {
        self.to_dot()
    }

    /// 将这个DFA最小化。
    ///
    /// 实现有点复杂。首先我们计算不可区分状态组`indistin_groups`，里面有几组不可区分状态。
    /// 先从原状态转移表中删除原有的不可区分状态，然后将每一组不可区分状态合并为一个状态，添加到表的末尾。
    ///
    /// 之后计算映射表`id_map`，将状态在旧表中的id映射为新表中的id。并且，同一组不可区分的状态会映射到同一个新id。
    /// 例如一组不可区分状态{q1，q2，q3}，那么这个映射表的记录就是：
    /// map(q1) = map(q2) = map(q3) = new_id。
    ///
    /// 极小化DFA的具体实现步骤如下：
    ///
    /// 0. 计算不可区分状态组和映射表。
    /// 1. 新建一个空的DFA。新DFA的状态数 = 原DFA的状态数 + 不可区分状态组的数量 - 不可区分状态数。
    /// 2. 合并不可区分状态组的转移函数并添加到新表中。理论上，因为组中的状态不可区分，它们的转移函数应该是一样的，只需取其中一个的信息即可。
    /// 3. 对于原DFA中的每一个状态转移函数δ(q,a)=p，
    ///     1. 如果q是不可区分状态组的成员，那么忽略这个δ。
    ///     2. 如果 p 是一个不可区分状态，将转移函数δ(q, a) = map(p)添加到极小化DFA中。
    ///     3. 如果 q 和 p 都不是不可区分状态，那么直接把δ(q,a)=p添加到新DFA中。
    /// 4. 把原DFA的初始状态和接收状态过一遍映射表，得到极小化DFA的初始状态和接收状态。
    pub fn minimize(&self) -> Option<Self> {
        let indistin_groups = minimize::compute_indistin_state_groups(self);
        if indistin_groups.num_of_groups() == 0 {
            return None;
        }
        let config = DfaConfig::new_for_minimize(self, &indistin_groups);
        let mut minimized_dfa = Self::init_with_config(&config);
        // dbg!(&minimized_dfa.accept_states);

        for old_state_id in 0..self.number_of_states() {
            if indistin_groups.contains_at(old_state_id).is_some() {
                continue;
            }
            let from = config.id_map[&old_state_id];
            for input in self.alphabet.to_iter() {
                let to = config.id_map[&self.delta(old_state_id, input)];
                minimized_dfa.add_transition(from, input, to);
            }
        }

        for group in indistin_groups.iter() {
            let old_id = group.iter().next().unwrap();
            let from = config.id_map[old_id];
            for input in self.alphabet.to_iter() {
                let to = config.id_map[&self.delta(*old_id, input)];
                dbg!(from, input, to);
                minimized_dfa.add_transition(from, input, to);
            }
        }

        Some(minimized_dfa)
    }
}

impl fmt::Display for DenseDFA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_fmt_output())
    }
}


/// 输入字符可以是任意ASCII码的稀疏DFA的状态。
///
/// 目前还没实现这样的DFA，所以这个结构体也没人用。
struct StateAscii {
    to: Vec<(u8, StateId)>,
}

impl State for StateAscii {
    type StateId = StateId;
    type Transitions = Vec<(u8, StateId)>;

    fn transitions(&self) -> Self::Transitions {
        self.to.clone()
    }
}
