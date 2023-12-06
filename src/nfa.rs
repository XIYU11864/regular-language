use itertools::Itertools;
use std::{collections::HashSet, iter::FromIterator};

// 这是一个正则语法解析相关的包，用于将正则表达式解析优化过的成语法树。
// 语法树的节点类型在regex_syntax::hir::HirKind中定义。
// 这个包实际上是rust语言的正则表达式库regex的一个子包，里面的算法是生产级的。
use regex_syntax::{
    hir::{self, Hir, HirKind::*},
    ParserBuilder,
};

// 使用u32作为状态索引让后续代码包含了无数的 StateId as usize 和 usize as StateId。
// 从一开始就不应该使用u32作为状态索引，应该使用usize，这样就不会有这种麻烦了。
type StateId = u32;

#[derive(Debug)]
pub struct NFA {
    states: Vec<State>,
    alphabet: HashSet<u8>,
    pub start_state: Option<StateId>,
    pub accept_states: Vec<StateId>,
}

/// NFA内的状态的增删改查
impl NFA {
    pub fn init_empty() -> NFA {
        NFA {
            states: Vec::new(),
            start_state: None,
            accept_states: Vec::new(),
            alphabet: HashSet::new(),
        }
    }

    pub fn add_state(&mut self, state: State) -> StateId {
        let id = self.states.len() as StateId;
        self.states.push(state);
        id
    }

    /// 添加一个空的、只能添加空转移的新状态。
    pub fn add_epsilon_state(&mut self) -> StateId {
        self.add_state(State::new_epsilon())
    }

    /// 添加一个空的、只能添加非空转移的新状态。
    pub fn add_non_epsilon_state(&mut self) -> StateId {
        self.add_state(State::new_non_epsilon())
    }

    /// 添加一个没有出路的新状态。
    pub fn add_fail_state(&mut self) -> StateId {
        self.add_state(State::new_fail())
    }

    /// 添加一个接收状态。
    pub fn add_final_state(&mut self) -> StateId {
        self.add_state(State::new_final())
    }

    pub fn add_transition(&mut self, from: StateId, input: u8, to: StateId) {
        if let State::NonEpsilon(trans) = &mut self.states[from as usize] {
            trans.0.push((input, to));
        } else {
            panic!(
                "add_transition: from state \"{}\" should be a non-epsilon state",
                from
            );
        }

        self.alphabet.insert(input);
    }

    pub fn add_epsilon_transition(&mut self, from: StateId, to: StateId) {
        if let State::Epsilon(trans) = &mut self.states[from as usize] {
            trans.0.push(to);
        } else {
            panic!(
                "add_epsilon_transition: from state \"{}\" should be a epsilon state",
                from
            );
        }
    }

    pub fn set_start_state(&mut self, state: StateId) {
        self.start_state = Some(state);
    }

    pub fn set_accept_state(&mut self, state: StateId) {
        self.accept_states.push(state);
    }

    pub fn reset_accept_states(&mut self) {
        self.accept_states.clear();
    }

    pub fn get_states_iter(&self) -> std::slice::Iter<State> {
        self.states.iter()
    }

    pub fn alphabet(&self) -> &HashSet<u8> {
        &self.alphabet
    }
}

/// 状态和转移的计算相关方法
impl NFA {
    /// 为了消除构造过程中产生的不必要的空转移，我们需要知道一个状态的入集。
    ///
    /// 本函数通过搜索整个NFA来获得一个状态的入集。
    /// 返回值是两个Vec，第一个代表能通过空转移来到此状态的状态集，第二个代表通过非空转移来到此状态的状态集。
    /// 我的NFA是结构像个单向链表，所以为了获得一个状态的入集（前导），需要遍历整个NFA。
    ///
    /// 我找到了不需要搜索入集也能消除不必要的状态的算法，所以这个函数目前不需要使用，太好了。
    fn search_inset_of_state(&self, state: StateId) -> (Vec<StateId>, Vec<(StateId, u8)>) {
        let mut epsilon_from = Vec::new();
        let mut non_epsilon_from = Vec::new();
        for (origin_id, origin_state) in self.states.iter().enumerate() {
            match origin_state {
                State::Epsilon(trans) => {
                    if trans.0.contains(&state) {
                        epsilon_from.push(origin_id as StateId);
                    }
                }
                State::NonEpsilon(trans) => {
                    for (input, to) in trans.iter() {
                        if *to == state {
                            non_epsilon_from.push((origin_id as StateId, *input));
                        }
                    }
                }
                _ => (),
            }
        }
        (epsilon_from, non_epsilon_from)

        // 注意，有另一个办法不需要遍历整个状态集合也能搜索入集。但是需要重构NFA的数据结构。
        //
        // 令状态转移函数不再储存于状态中，而是全部存放在一个总的Vec里。
        // 这个大Vec的元素是 `(u8, StateId)` ，也就是一个状态转移函数。
        // 如何知道转移函数的起始状态呢？把整个Vec看做一个个长度相等的片段，每个片段的长度等于NFA的字母表的长度。
        // 每一个片段相当于储存了某个特定状态的状态转移表。
        // 这样当我们需要搜索一个状态的入集，就可以用“跳步”的方法来访问这个大Vec。
        // 每次访问都跨越字母表的大小个长度。这样只需要O(n)复杂度即可找到一个状态的入集，n是NFA中的状态数量。
        // 而对于当前使用的结构，这个复杂度最坏是O(n^2)。
        //
        // 这个结构的缺点是一个输入字符只能记录一个目标状态。
        // 但是，教材使用的 thompson 构造法来构造NFA，这个方法不会出现一个输入字符指向多个状态的情况，除非是空转移。
        // 但同时，这个构造法也使得某个状态要么只包含空转移，要么只包含非空转移，所以处理空转移也很方便。
        //
        // 由于我们的题目所构造的NFA状态数不会太多，所以暂时就用现在的结构了。
    }

    /// 这个函数的意义是，先求状态的闭包，然后再求从闭包中任意状态发射的所有非空转移。
    fn epsilon_closure_and_dalta(&self, state: StateId) -> (Vec<StateId>, HashSet<(u8, u32)>) {
        let mut closure = Vec::new();
        let mut stack = vec![state];
        let mut target = HashSet::new();
        while let Some(state) = stack.pop() {
            closure.push(state);
            match &self.states[state as usize] {
                State::Epsilon(trans) => {
                    for to in trans.iter() {
                        if !closure.contains(to) {
                            stack.push(*to);
                        }
                    }
                }
                State::NonEpsilon(trans) => {
                    for tran in trans.iter() {
                        target.insert(*tran);
                    }
                }
                State::Fail | State::Final => (),
            }
        }
        (closure, target)
    }

    /// 本函数的意义是求状态的闭包，但是只返回闭包中的非空状态`State::NonEpsilon`。
    fn epsilon_closure_to_non_epsilon(&self, state: StateId) -> HashSet<StateId> {
        let mut closure = HashSet::new();
        let mut stack = vec![state];
        let mut target = HashSet::new();
        while let Some(state) = stack.pop() {
            closure.insert(state);
            match &self.states[state as usize] {
                State::Epsilon(trans) => {
                    for to in trans.iter() {
                        if !closure.contains(to) {
                            stack.push(*to);
                        }
                    }
                }
                State::NonEpsilon(_) | State::Fail | State::Final => {
                    target.insert(state);
                }
            }
        }
        target
    }

    // 千万别随便用递归，容易栈溢出！！
    // fn epsilon_closure_recursively(&self, state: StateId) -> HashSet<StateId> {
    //     let mut closure = HashSet::new();
    //     if let State::Epsilon(trans) = &self.states[state as usize] {
    //         for id in trans.iter() {
    //             closure.insert(*id);
    //             closure.extend(self.epsilon_closure_recursively(*id));
    //         }
    //     } else {
    //         closure.insert(state);
    //     }
    //     closure
    // }

    /// 以分组的形式返回某个非空转移状态的所有转移，同一个输入字符能达到的状态分到同一个组中。
    pub fn deltas(&self, state_id: StateId) -> Vec<(u8, Vec<StateId>)> {
        if let State::NonEpsilon(trans) = &self.states[state_id as usize] {
            trans
                .iter()
                .sorted_by(|(input1, _), (input2, _)| input1.cmp(input2))
                .group_by(|(input, _)| input)
                .into_iter()
                .map(|(input, group)| (*input, group.map(|(_, to)| *to).collect()))
                .collect()
        } else {
            Vec::new()
        }
        // todo!()
    }

    /// 返回“delta hat"转移函数，即去除空转移后的转移函数。
    fn get_dalta_hat_transitions(&self, state: StateId) -> Vec<(u8, u32)> {
        let mut result = Vec::new();

        let (_, non_epsilon_transet) = self.epsilon_closure_and_dalta(state);
        for (input, to) in non_epsilon_transet {
            self.epsilon_closure_to_non_epsilon(to)
                .iter()
                .for_each(|s| result.push((input, *s)));
        }
        result
    }

    /// 搜索不可达状态。此函数可能复杂度很高。
    fn search_unreachable_states(&self) -> HashSet<StateId> {
        let mut reachable_states = HashSet::new();
        let mut stack = Vec::new();
        stack.push(self.start_state.unwrap());

        let mut times = 0; // 用于调试，记录搜索次数。

        while let Some(state) = stack.pop() {
            if reachable_states.insert(state) {
                if let State::NonEpsilon(trans) = &self.states[state as usize] {
                    for (_, next_state) in trans.iter() {
                        stack.push(*next_state);
                        times += 1;
                    }
                }
            }
        }
        dbg!(times);

        HashSet::from_iter(0 as StateId..self.states.len() as StateId)
            .difference(&reachable_states)
            .cloned()
            .collect()
    }

    /// 重新建立状态集合的索引，去除fail状态。
    /// 只应该在已去除空转移的NFA上调用！
    fn remap_states(&mut self) {
        // 生成一个从旧状态编号到新状态编号的映射表。
        let mut id_map = Vec::with_capacity(self.states.len());

        // 新状态编号从1开始。DFA需要把0号状态作为陷阱状态，如果在NFA中就预留出0号状态的位置，构造DFA会比较方便。
        // ↑错误的，不需要从1开始。因为DFA的幂集构造法自然包含一个空子集，编号恰好是0。
        let mut new_index: StateId = 0;
        for state in self.states.iter() {
            match state {
                State::Epsilon(_) | State::NonEpsilon(_) | State::Final => {
                    id_map.push(Some(new_index));
                    new_index += 1;
                }
                State::Fail => id_map.push(None),
            }
        }

        for id in 0..self.states.len() {
            self.remap_trans(id as StateId, &id_map);
        }

        for (old, new) in id_map.iter().enumerate().rev() {
            if let None = new {
                self.states.remove(old);
            }
            // dbg!((old, new));
        }
        // 最后在状态表的开头插入一个元素，让原来的所有元素的索引都+1，以预留出0号状态。
        // self.states.insert(0, State::Fail);
        // 还需要把开始状态和结束状态编号+1。
        // self.start_state = self.start_state.map(|id| id + 1);
        // self.accept_states = self
        //     .accept_states
        //     .iter()
        //     .map(|id| id + 1)
        //     .collect::<Vec<StateId>>();
        // 最后状态列表中应该有一个陷阱状态，一个接收状态，其他都是非空转移状态。

        // 最后状态列表中应该只有一个接收状态，其他都是非空转移状态。
    }

    fn remap_trans(&mut self, state: StateId, map: &Vec<Option<StateId>>) {
        if let State::NonEpsilon(ref mut trans) = &mut self.states[state as usize] {
            trans.0 = trans
                .iter()
                .map(|(input, to)| (*input, map[*to as usize].expect("map to a fail state")))
                .collect();
        }
    }
}

/// 一些开发时的测试
impl NFA {
    pub fn test_print_alphabet(&self) {
        for ele in &self.alphabet {
            println!("{}", *ele as char);
        }
    }

    /// 用于测试，打印NFA的所有状态的epsilon闭包。
    pub fn test_print_closure(&self) {
        for (id, _) in self.states.iter().enumerate() {
            println!(
                "{}: {:?}",
                id,
                self.epsilon_closure_to_non_epsilon(id as StateId)
            );
        }
    }

    pub fn test_print_inset_of_state(&self, id: StateId) {
        dbg!(self.search_inset_of_state(id));
    }
}

/// 格式化相关方法
impl NFA {
    // 此方法由copilot生成，👍
    // 生成dot文件，可以由graphviz生成状态机图
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph {\n");
        dot.push_str("rankdir=LR;\n");
        // dot.push_str("size=\"8,5\";\n");
        dot.push_str("node [shape = doublecircle];\n");
        for state in &self.accept_states {
            dot.push_str(&format!("{};\n", state));
        }
        dot.push_str("node [shape = circle];\n");
        for (id, state) in self.states.iter().enumerate() {
            match state {
                State::Epsilon(trans) => {
                    for to in trans.iter() {
                        dot.push_str(&format!("{} -> {} [label=\"ε\"];\n", id, to))
                    }
                }
                State::NonEpsilon(trans) => {
                    for (input, to) in trans.iter() {
                        dot.push_str(&format!(
                            "{} -> {} [label=\"{}\"];\n",
                            id, to, *input as char
                        ))
                    }
                }

                State::Final | State::Fail => {}
            }
        }
        dot.push_str("}");
        dot
    }
}

/// NFA的状态类型，有三种：
/// 1. Epsilon，只能添加空转移的状态。
/// 2. NonEpsilon，只能添加非空转移的状态。
/// 3. NoWayOut，没有出路的状态。
///
/// thompson 构造法构造NFA，状态要么包含空转移，要么包含非空转移，不会同时包含两种转移，因此这么设计是可以的。
/// 这么做的目的是为了方便后续计算空闭包。
/// 另外，NoWayOut类状态可以用作接收状态或者陷阱状态。
#[derive(Debug)]
pub enum State {
    Epsilon(EpsilonTrans),
    NonEpsilon(NonEpsilonTrans),

    /// 将NoWayOut进一步细化为了两种状态，fail代表陷阱状态，final代表接收状态，方便后续计算。
    Fail,
    Final,
}
#[derive(Debug, Clone)]
pub struct EpsilonTrans(Vec<StateId>);

impl EpsilonTrans {
    pub fn iter(&self) -> std::slice::Iter<StateId> {
        self.0.iter()
    }
}
#[derive(Debug, Clone)]
pub struct NonEpsilonTrans(Vec<(u8, StateId)>);

impl NonEpsilonTrans {
    pub fn iter(&self) -> std::slice::Iter<(u8, StateId)> {
        self.0.iter()
    }
}
impl State {
    pub fn new_epsilon() -> State {
        State::Epsilon(EpsilonTrans(Vec::new()))
    }
    pub fn new_non_epsilon() -> State {
        State::NonEpsilon(NonEpsilonTrans(Vec::new()))
    }
    pub fn new_fail() -> State {
        State::Fail
    }
    pub fn new_final() -> State {
        State::Final
    }
}

/// NFA的构造器，在这里实现一个visitor，用于遍历正则表达式的语法树。
/// thompson 构造法构造NFA，有两种思路：
///
/// 1. 自底向上，先构造子NFA，记录每一个子NFA的开始和接受状态，然后把子NFA合并成一个大NFA。
/// 2. 自顶向下，从AST的根节点开始直接构造NFA，用“空穴”代替子NFA，记录空穴的“来源”和“去路”。构造子NFA时填入空穴。
///
/// 这里我用的是第二种思路。一般来说用自底向上方法，递归地构造NFA，比较直观。
/// 但是如果需要构造的NFA很大，例如AST深度达到1000层以上，递归函数的调用栈可能会溢出。
/// 所以尝试使用自顶向下的方法，用栈来辅助NFA的构造过程。
/// 虽然这样会严重降低代码的可读性，但其实也不会有人看我的代码。
pub struct Builder {
    nfa: NFA,
    stack: Vec<Hole>,
}

/// 用于创建NFA时使用的栈的单个栈帧，aka“空穴”。
/// 每当进入一个节点时，取出一个栈帧，获得从这个节点构造的子NFA的“来源”和“去路”。
/// 然后在离开这个节点时，将子节点需要的栈帧压入栈中。
#[derive(Debug)]
enum Hole {
    Alternation { come_from: StateId, go_to: StateId },
    Concatenation { come_from: StateId, go_to: StateId },
    Repetition { come_from: StateId, go_to: StateId },
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            nfa: NFA::init_empty(),
            stack: Vec::new(),
        }
    }

    pub fn build_nfa_from_re(mut self, re: &String) -> Result<NFA, String> {
        let hir = ParserBuilder::new()
            .unicode(false)
            .utf8(false)
            .build()
            .parse(re)
            .unwrap();
        // parse(re).unwrap();
        // let start = self.nfa.add_epsilon_state();
        let end = self.nfa.add_fail_state();

        self.nfa.set_accept_state(end);

        let start = self.nfa.add_epsilon_state();
        self.nfa.set_start_state(start);

        self.stack.push(Hole::Alternation {
            come_from: start,
            go_to: end,
        });

        // dbg!(&hir);

        hir::visit(&hir, self)
    }

    /// 构造没有空转移的NFA
    pub fn build_non_epsilon_nfa(mut self, old_nfa: &NFA) -> Result<NFA, String> {
        // 第一步，将状态转移函数dalta转换成dalta_hat

        // 首先将原NFA中的状态全部添加到新NFA中。
        for state_id in 0..old_nfa.states.len() {
            let trans = old_nfa.get_dalta_hat_transitions(state_id as StateId);
            if trans.is_empty() {
                if old_nfa.accept_states.contains(&(state_id as StateId)) {
                    self.nfa.add_final_state();
                } else {
                    self.nfa.add_fail_state();
                }
                println!("empty {}", state_id);
            } else {
                self.nfa.add_non_epsilon_state();
                // 如果一边添加状态一边添加转移函数，最后不得不进行复杂的删除陷阱状态的步骤。
                // 因为添加状态的过程中无法区分一个状态是否是陷阱状态。
                // for (input, to) in trans.iter() {
                //     self.nfa.add_transition(new_state, *input, *to);
                // }
            }
        }

        // 然后把原NFA的所有状态转移函数dalta转化为dalta_hat并添加到新NFA中。
        for state_id in 0..old_nfa.states.len() {
            if let State::NonEpsilon(_) = &self.nfa.states[state_id] {
                let trans = old_nfa.get_dalta_hat_transitions(state_id as StateId);
                for (input, to) in trans.iter() {
                    if let State::Fail = &self.nfa.states[*to as usize] {
                        continue;
                    }
                    self.nfa.add_transition(state_id as StateId, *input, *to);
                }
            }
        }

        self.nfa.set_start_state(old_nfa.start_state.unwrap());
        self.nfa.set_accept_state(old_nfa.accept_states[0]);

        // 下一步删除不可达状态
        for unreachable_state_id in self.nfa.search_unreachable_states() {
            self.nfa.states[unreachable_state_id as usize] = State::Fail;
        }
        // dbg!(self.nfa.states.len());
        self.nfa.remap_states();

        // dbg!(self.nfa.states.len());

        // 删除陷阱状态，不需要了
        // for id in 0..self.nfa.states.len() {
        //     if let State::Final = self.nfa.states[id] {
        //         if self.nfa.accept_states.contains(&(id as StateId)) {
        //             continue;
        //         }
        //         let (_, inset) = self.nfa.search_inset_of_state(id as StateId);
        //         for (from_state, _) in inset {
        //             if let State::NonEpsilon(trans) = &mut self.nfa.states[from_state as usize] {
        //                 trans.0.retain(|(_, e)| *e != id as StateId);
        //             }
        //         }
        //         self.nfa.states[id] = State::Fail;
        //     }
        // }

        Ok(self.nfa)
    }
}

impl regex_syntax::hir::Visitor for Builder {
    type Output = NFA;
    type Err = String;

    fn start(&mut self) {}

    /// 访问AST的一个节点。
    fn visit_pre(&mut self, _hir: &Hir) -> Result<(), Self::Err> {
        // 第一步，生成这个节点对应的子NFA的结束节点
        let end = self.nfa.add_epsilon_state();

        // 第二步，获得此子NFA的入口和出口
        let hole = self.stack.pop();
        let (come_from, go_to) = match hole {
            Some(Hole::Concatenation { come_from, go_to }) => {
                self.stack.push(Hole::Concatenation {
                    come_from: end,
                    go_to,
                });
                (come_from, go_to)
            }
            Some(Hole::Alternation { come_from, go_to })
            | Some(Hole::Repetition { come_from, go_to }) => (come_from, go_to),
            None => return Err("stack is empty".to_string()),
        };

        // 第三步，生成子NFA的开始节点，并根据节点类型，生成子NFA，
        let start = match _hir.kind() {
            //连接
            Concat(_) => {
                let start = self.nfa.add_epsilon_state();
                // self.nfa.add_epsilon_transition(come_from, start);
                self.stack.push(Hole::Concatenation {
                    come_from: start,
                    go_to: end,
                });
                start
            }
            //或
            Alternation(sub_hirs) => {
                let start = self.nfa.add_epsilon_state();
                // self.nfa.add_epsilon_transition(come_from, start);
                for _ in 0..sub_hirs.len() {
                    self.stack.push(Hole::Alternation {
                        come_from: start,
                        go_to: end,
                    });
                }
                start
            }

            //字符串。在AST中，连续地对字符进行连接会被合并成一个Literal节点。
            //例如“001+11001*0”这个RE，会生成“001”“1100”这样的Literal节点，而不是Concat(["0","0","1"])这样的Concat节点。
            Literal(literal) => {
                let start = self.nfa.add_non_epsilon_state();

                let mut current = start;
                let len = literal.0.len();
                let mut iter = literal.0.iter().peekable();
                for _ in 0..len {
                    let c = iter.next().unwrap();
                    if let Some(_) = iter.peek() {
                        let new_state = self.nfa.add_non_epsilon_state();
                        self.nfa.add_transition(current, *c, new_state);
                        current = new_state;
                    } else {
                        self.nfa.add_transition(current, *c, end);
                    }
                }
                start
                // self.nfa.add_epsilon_transition(current, end);
            }

            //单个字符的或，比如 "1|2|3|0" 会被构造成 Class({'0'..='3'})
            // "1|2|3|8|9|8|7|5" 会构造成 Class({'1'..='3', '5'..='5', '7'..='9'})
            // 在原包中，这是为了支持真正的正则表达式的范围语法[0-9]等。
            Class(class) => {
                let start = self.nfa.add_non_epsilon_state();

                macro_rules! add_range_trans {
                    ($range_set:expr, $start:expr, $end:expr, $nfa:expr) => {
                        for range in $range_set.iter() {
                            for c in range.start()..=range.end() {
                                $nfa.add_transition($start, c as u8, $end);
                            }
                        }
                    };
                }
                match class {
                    hir::Class::Bytes(range_set) => {
                        add_range_trans!(range_set, start, end, self.nfa)
                    }

                    hir::Class::Unicode(range_set) => {
                        add_range_trans!(range_set, start, end, self.nfa)
                    }
                }
                start
            }

            //重复，即闭包操作符*。regex_syntax包还支持正闭包+、非贪婪闭包*?、非贪婪正闭包+?等其他重复语法。
            Repetition(r) => {
                // 我们只用克林闭包操作符*。如果出现了别的情况，说明输入的RE有错误，直接panic！
                assert!(r.greedy && r.min == 0 && r.max.is_none());

                let start = self.nfa.add_epsilon_state();
                self.nfa.add_epsilon_transition(start, end);
                self.stack.push(Hole::Repetition {
                    come_from: start,
                    go_to: end,
                });
                start
            }
            //捕获，可以当作括号
            Capture(_) => {
                let start = self.nfa.add_epsilon_state();
                self.stack.push(Hole::Alternation {
                    come_from: start,
                    go_to: end,
                });
                start
            }
            //空串，代表一个接受空语言的正则表达式。
            Empty => {
                let start = self.nfa.add_epsilon_state();
                self.nfa.add_epsilon_transition(start, end);
                start
            }
            //在教材里的正则表达式语法中不会出现
            Look(_) => {
                return Err("unexpected \"Look\" syntax".to_string());
            }
        };

        // 第四步，收尾工作，将子NFA的填入“空穴”中。
        // 如果这个“空穴”代表闭包操作符*的子NFA，还需要添加一个从子NFA的结束节点到开始节点的空转移。
        self.nfa.add_epsilon_transition(come_from, start);
        // self.nfa.add_epsilon_transition(end, go_to);

        match hole {
            Some(Hole::Repetition {
                come_from: _,
                go_to: _,
            }) => {
                self.nfa.add_epsilon_transition(end, go_to);
                self.nfa.add_epsilon_transition(end, start);
            }
            Some(Hole::Alternation {
                come_from: _,
                go_to: _,
            }) => {
                self.nfa.add_epsilon_transition(end, go_to);
            }
            _ => (),
        }
        Ok(())
    }

    // 访问完一个节点的所有子节点之后调用本函数。
    // 有个bug，根节点不会调用这个方法。
    fn visit_post(&mut self, _hir: &Hir) -> Result<(), Self::Err> {
        if let Concat(_) = _hir.kind() {
            if let Some(Hole::Concatenation { come_from, go_to }) = self.stack.pop() {
                self.nfa.add_epsilon_transition(come_from, go_to);
            }
        }
        Ok(())
    }

    fn visit_alternation_in(&mut self) -> Result<(), Self::Err> {
        Ok(())
    }

    fn visit_concat_in(&mut self) -> Result<(), Self::Err> {
        Ok(())
    }

    /// 本方法会消费掉这个builder自己，然后返回构造完毕的NFA。
    fn finish(mut self) -> Result<Self::Output, Self::Err> {
        if let Some(Hole::Concatenation { come_from, go_to }) = &self.stack.pop() {
            self.nfa.add_epsilon_transition(*come_from, *go_to);
        }
        dbg!(&self.stack);
        Ok(self.nfa)
    }
}
